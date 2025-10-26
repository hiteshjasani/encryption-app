use std::path::PathBuf;

use iced::{
    alignment::{Horizontal, Vertical}, color, widget::{column, container, horizontal_rule, row, text, Text, text_input}, Element, Font, Task
};
use iced_modern_theme::Modern;
use iced_optional_element_shim::to_elem;

use foo::FileMeta;

fn main() -> iced::Result {
    iced::application("encryption-app", App::update, App::view)
        .theme(|_app| iced_modern_theme::Modern::dark_theme())
        // .theme(|_app| Theme::SolarizedDark)
        // .theme(|_app| Theme::TokyoNight)
        // .theme(|_app| Theme::TokyoNightStorm)
        .default_font(Font::MONOSPACE)
        .run_with(App::new)
}

struct App {
    directory: String,
    filelist: Vec<FileMeta>,
}

#[derive(Debug, Clone)]
enum Message {
    RefreshList,
    DirectoryChanged(String),
    FileList(Result<Vec<FileMeta>, Error>),
    Action(usize, foo::Message),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                directory: String::from("/tmp"),
                filelist: Vec::new(),
            },
            Task::done(Message::RefreshList)
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RefreshList => {
                Task::perform(list_files(PathBuf::from(self.directory.as_str())),
                Message::FileList)
            }
            Message::DirectoryChanged(new_dir) => {
                self.directory = new_dir;
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
                        .then(|_| Task::done(Message::RefreshList))
                } else {
                    Task::none()
                }
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let icon = iced_font_awesome::fa_icon("folder-open").size(16.0).color(color!(249, 170, 51));
        let label = text("Enter directory:");
        let dir_input = text_input("Directory", self.directory.as_str())
            .style(Modern::text_input())
            .on_input(Message::DirectoryChanged)
            .width(600);
        let input_col = column!(
            row!(icon, label).spacing(10).align_y(Vertical::Center),
            dir_input
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
            filecol
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

    use iced::{alignment::Vertical, widget::{button, row, text, Row, Space, Text}, Element, Task};
    use iced_modern_theme::Modern;
    use iced_optional_element_shim::to_elem;
    use tokio::{
        fs::File,
        io::AsyncWriteExt,
    };

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
                println!("encrypting {} to {}", file_meta.name, enc_filename.display());
                Task::future(async move {
                    let _ = write_file(&enc_filename, "hello world encrypted").await;
                    Message::FileSystemUpdated
                    // super::Message::RefreshList
                })
            }
            Message::Decrypt => {
                println!("decrypt {}", file_meta.name);
                Task::future(async move {
                    Message::FileSystemUpdated
                })
            }
            Message::Delete => {
                println!("delete {}", file_meta.name);
                Task::future(async move {
                    Message::FileSystemUpdated
                })
            }
            Message::FileSystemUpdated => {
                Task::none()
            }
        }
    }

    pub fn view(file_meta: &FileMeta) -> Element<Message> {
        let is_file = file_meta.is_file;
        let is_enc_file = is_file && is_encrypted(&file_meta.path);
        row!(
            text(&file_meta.name).width(180),
            text(file_meta.type_as_str()).width(50),

            if is_enc_file {
                to_elem(Some(button(text("encrypt"))
                    .style(Modern::blue_tinted_button())))
            } else {
                to_elem(Some(button(text("encrypt"))
                    .style(Modern::primary_button())
                    .on_press(Message::Encrypt)))
            },

            if is_enc_file {
                to_elem(Some(button(text("decrypt"))
                    .style(Modern::warning_button())
                    .on_press(Message::Decrypt)))
            } else {
                to_elem(Some(button(text("decrypt"))
                    .style(Modern::warning_button())))
            },

            if is_file {
                row!(
                    Space::with_width(80),
                    button(text("delete"))
                        .style(Modern::danger_button())
                        .on_press(Message::Delete)
                )
            } else {
                row!(to_elem::<Message, Text>(None))
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
    }
}

