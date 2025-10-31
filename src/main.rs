use std::path::PathBuf;

use iced::{
    alignment::{Horizontal, Vertical}, color, widget::{button, column, container, horizontal_rule, row, scrollable, text, text_input, Text}, Element, Font, Task
};
use iced_font_awesome as ifa;
use iced_modern_theme::Modern;
use iced_optional_element_shim::to_elem;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use foo::FileMeta;

mod crypto;

fn main() -> iced::Result {
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::INFO)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    iced::application("encryption-app", App::update, App::view)
        .theme(|_app| iced_modern_theme::Modern::dark_theme())
        // .theme(|_app| Theme::SolarizedDark)
        // .theme(|_app| Theme::TokyoNight)
        // .theme(|_app| Theme::TokyoNightStorm)
        .default_font(Font::MONOSPACE)
        .run_with(App::new)
}

struct App {
    directory: PathBuf,
    filelist: Vec<FileMeta>,
}

#[derive(Debug, Clone)]
enum Message {
    RefreshList,
    DirectoryChanged(String),
    DirectoryDown(String),
    DirectoryUp,
    FileList(Result<Vec<FileMeta>, Error>),
    Action(usize, foo::Message),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                directory: PathBuf::from("/tmp"),
                filelist: Vec::new(),
            },
            Task::done(Message::RefreshList)
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RefreshList => {
                Task::perform(list_files(self.directory.clone()),
                Message::FileList)
            }
            Message::DirectoryChanged(new_dir) => {
                self.directory = PathBuf::from(new_dir);
                Task::done(Message::RefreshList)
            }
            Message::DirectoryDown(new_dir) => {
                self.directory.push(new_dir);
                Task::done(Message::RefreshList)
            }
            Message::DirectoryUp => {
                self.directory.pop();
                Task::done(Message::RefreshList)
            }
            Message::FileList(result) => {
                if let Ok(mut files) = result {
                    files.sort_by_key(|x| x.name.clone());
                    self.filelist = files;
                }
                Task::none()
            }
            Message::Action(index, fm_message) => {
                if let Some(filemeta) = self.filelist.get_mut(index) {
                    foo::update(filemeta, fm_message)
                        .then(|fm_msg| {
                            match fm_msg {
                                foo::Message::LinkClicked(url) => {
                                    info!("Should be changing dir to {}", url);
                                    Task::done(Message::DirectoryDown(url))
                                }
                                _ => Task::done(Message::RefreshList)
                            }
                        })
                } else {
                    Task::none()
                }
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let icon = ifa::fa_icon("folder-open").size(16.0).color(color!(249, 170, 51));
        let label = text("Enter directory:");
        let dir_input = text_input("Directory", self.directory.to_str().unwrap_or_else(|| "."))
            .style(Modern::text_input())
            .on_input(Message::DirectoryChanged)
            .width(600);
        let input_col = column!(
            row!(icon, label).spacing(10).align_y(Vertical::Center),
            row!(
                button(ifa::fa_icon_solid("arrow-up").size(16.0)).on_press(Message::DirectoryUp),
                dir_input
            ).spacing(10).align_y(Vertical::Center)
        )
            .align_x(Horizontal::Left)
            .spacing(10)
            .padding(10);
        let input_ctr = container(input_col)
            .padding(10)
            .style(Modern::sheet_container());

        let filecol = column(
            self.filelist
                .iter()
                .map(foo::view)
                .enumerate()
                .map(|(index, filemeta)| {
                    filemeta.map(move |message| Message::Action(index, message))
                })
        ).spacing(10);
        column!(
            input_ctr,
            horizontal_rule(2),

            if true {
                to_elem(Some(text(format!("if true succeeded"))))
            } else {
                to_elem::<Message, Text>(None)
            },
            if false {
                to_elem(Some(text(format!("if false failed"))))
            } else {
                to_elem::<Message, Text>(None)
            },


            horizontal_rule(2),
            scrollable(filecol)
        )
        .padding(30)
        .spacing(20)
        .into()
    }
}

async fn list_files(base: PathBuf) -> Result<Vec<FileMeta>, Error> {
    let mut entries = tokio::fs::read_dir(base).await
        .map_err(|e| Error::IoError(format!("Unable to read dir: {e}")))?;
    let mut files = Vec::new();

    while let Some(entry) = entries.next_entry().await
            .map_err(|e| Error::IoError(format!("Error getting dir entries: {e}")))? {

        let name = entry.file_name().display().to_string();
        // let pathbuf = entry.path();
        let filetype = entry.file_type().await
            .map_err(|e| Error::IoError(format!("Error getting file type: {e}")))?;
        let is_dir = filetype.is_dir();
        let is_file = filetype.is_file();
        let is_symlink = filetype.is_symlink();
        let ino = entry.ino();
        let path = entry.path();
        let fm = FileMeta {
            name,
            is_dir,
            is_file,
            is_symlink,
            ino,
            path,
        };
        files.push(fm);
    }

    Ok(files)
}

#[derive(Debug, Clone)]
pub enum Error {
    IoError(String),
}

mod foo {
    use std::path::PathBuf;

    use iced::{alignment::Vertical, color, widget::{button, rich_text, row, span, text, Space, Text}, Element, Task};
    use iced_font_awesome as ifa;
    use iced_modern_theme::Modern;
    use iced_optional_element_shim::to_elem;
    use tokio::{
        fs::File,
        io::AsyncWriteExt,
    };
    use tracing::info;

    #[allow(dead_code)]
    #[derive(Debug, Clone, Default)]
    pub struct FileMeta {
        pub name: String,
        pub is_dir: bool,
        pub is_file: bool,
        pub is_symlink: bool,
        pub ino: u64,
        pub path: PathBuf,
    }

    impl FileMeta {
        pub fn type_as_str<'a>(&self) -> &'a str {
            if self.is_dir {
                "dir"
            } else if self.is_symlink {
                "symlink"
            } else if self.is_file {
                "file"
            } else {
                "???"
            }
        }
    }

    pub fn update(file_meta: &mut FileMeta, message: Message) -> Task<Message> {
        match message {
            Message::Encrypt => {
                let enc_filename = gen_encrypted_filename(&file_meta.path);
                info!("encrypting {} to {}", file_meta.name, enc_filename.display());
                Task::future(async move {
                    let _ = write_file(&enc_filename, "hello world encrypted").await;
                    Message::FileSystemUpdated
                    // super::Message::RefreshList
                })
            }
            Message::Decrypt => {
                info!("decrypt {}", file_meta.name);
                Task::future(async move {
                    Message::FileSystemUpdated
                })
            }
            Message::Delete => {
                info!("delete {}", file_meta.name);
                Task::future(async move {
                    Message::FileSystemUpdated
                })
            }
            Message::FileSystemUpdated => {
                Task::none()
            }
            Message::LinkClicked(url) => {
                info!("Clicked link to {}", &url);
                // wrap message in task so parent can process event
                Task::future(async move {
                    Message::LinkClicked(url)
                })
            }
        }
    }

    pub fn view(file_meta: &FileMeta) -> Element<'_, Message> {
        let is_file = file_meta.is_file;
        let is_dir = file_meta.is_dir;
        let is_symlink = file_meta.is_symlink;
        let is_enc_file = is_file && is_encrypted(&file_meta.path);
        let is_key_file = is_file && is_keyfile(&file_meta.path);
        let text_color = if is_dir {
                Some(color!(80, 80, 255))
            } else if is_symlink {
                Some(color!(255, 255, 0))
            } else if is_file {
                Some(color!(200, 200, 200))
            } else {
                None
            };
        let link = if is_dir {
            Some(file_meta.name.clone())
        } else {
            None
        };

        row!(
            if is_enc_file {
                to_elem(Some(ifa::fa_icon_solid("lock").size(16.0).color(color!(255, 0, 0))))
            } else if is_key_file {
                to_elem(Some(ifa::fa_icon_solid("key").size(16.0).color(color!(0, 255, 0))))
            } else {
                to_elem(Some(ifa::fa_icon_solid("lock-open").size(16.0)))
                // to_elem(Some(Space::with_width(16)))
                // to_elem::<Message, Text>(None)
            },

            // Display file or directory name
            // text(&file_meta.name).width(300),
            rich_text([span(&file_meta.name).color_maybe(text_color).link_maybe(link.map(Message::LinkClicked)).into()]).width(300),

            text(file_meta.type_as_str()).width(50),

            if is_enc_file {
                to_elem(Some(button(text("encrypt"))
                    .style(Modern::blue_tinted_button())))
            } else {
                to_elem(Some(button(text("encrypt"))
                    .style(Modern::primary_button())
                    .on_press(Message::Encrypt)))
                    // .on_press(Some(Message::Encrypt))))
            },

            if is_enc_file {
                to_elem(Some(button(text("decrypt"))
                    .style(Modern::warning_button())
                    .on_press(Message::Decrypt)))
                    // .on_press(Some(Message::Decrypt))))
            } else {
                to_elem(Some(button(text("decrypt"))
                    .style(Modern::warning_button())))
            },

            if is_file {
                row!(
                    Space::with_width(80),
                    button(text("delete"))
                        .style(Modern::danger_button())
                        .on_press(Message::Delete),
                        // .on_press(Some(Message::Delete)),
                    Space::with_width(30)
                )
            } else {
                row!(to_elem::<Message, Text>(None))
                // row!(to_elem::<Option<Message>, Text>(None))
            }
        )
            .align_y(Vertical::Center)
            .spacing(10).into()
    }

    fn gen_encrypted_filename(pb: &PathBuf) -> PathBuf {
        let mut npb = PathBuf::new();
        if let Some(parent) = pb.parent() {
            npb = npb.join(parent);
        }
        if let Some(file_stem) = pb.file_stem() {
            npb = npb.join(format!("{}_enc", file_stem.display()));
        } else {
            npb = npb.join("enc");
        }
        if let Some(extension) = pb.extension() {
            let _ = npb.set_extension(extension);
        }
        npb
    }

    fn is_encrypted(pb: &PathBuf) -> bool {
        if let Some(file_stem) = pb.file_stem() {
            file_stem.display().to_string().ends_with("_enc")
        } else {
            false
        }
    }

    fn is_keyfile(pb: &PathBuf) -> bool {
        if let Some(file_stem) = pb.file_stem() {
            file_stem.display().to_string().ends_with("_key")
        } else {
            false
        }
    }

    async fn write_file(filepath: &PathBuf, content: &str) -> Option<bool> {
        (async move || {
            let mut file = File::create(filepath).await?;
            file. write_all(content.as_bytes()).await?;
            file.flush().await?;

            Ok::<bool, Box<dyn std::error::Error>>(true)
        })()
        .await.ok()
    }

    #[derive(Debug, Clone)]
    pub enum Message {
        Encrypt,
        Decrypt,
        Delete,
        FileSystemUpdated,
        LinkClicked(String),
    }
}

