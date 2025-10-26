use iced::{advanced::{layout, mouse, renderer, widget::Tree, Layout, Widget}, Element, Length, Rectangle, Renderer, Size, Theme};

/// Works to convert Option<T> to Element<'a, Message, Theme, Renderer>
/// iced versions after 0.13.1 have a From impl so they don't need this
/// Usage:
///         column!(
///            input_ctr,
///            horizontal_rule(2),
///
///            if true {
///                to_elem(Some(text(format!("if true succeeded"))))
///            } else {
///                to_elem::<Message, iced::widget::Text>(None)
///            },
///            if false {
///                to_elem(Some(text(format!("if false failed"))))
///            } else {
///                to_elem::<Message, iced::widget::Text>(None)
///            },
///
///
///            horizontal_rule(2),
///            filecol
///        )
pub fn to_elem<'a, Message, T: Into<Element<'a, Message, Theme, Renderer>>>(element: Option<T>) -> Element<'a, Message, Theme, Renderer> {
    struct Void;

    impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Void
    where Renderer: iced::advanced::Renderer
    {
        fn size(&self) -> Size<Length> {
            Size {
                width: Length::Fixed(0.0),
                height: Length::Fixed(0.0),
            }
        }

        fn layout(
            &self,
            _tree: &mut Tree,
            _renderer: &Renderer,
            _limits: &layout::Limits,
        ) -> layout::Node {
            layout::Node::new(Size::ZERO)
        }

        fn draw(
            &self,
            _tree: &Tree,
            _renderer: &mut Renderer,
            _theme: &Theme,
            _style: &renderer::Style,
            _layout: Layout<'_>,
            _cursor: mouse::Cursor,
            _viewport: &Rectangle,
        ) {
        }
    }

    element.map(T::into).unwrap_or_else(|| Element::new(Void))
}

/*
// compiles but runs into problems on usage
//
//            if true {
//                to_elem(Some(text(format!("if true succeeded"))))
//            } else {
//                to_elem::<Element<Message, iced::Theme, iced::Renderer>>(None)
//            },
//            if false {
//       ->>      to_elem(Some(text(format!("if false failed"))))
//            } else {
//                to_elem(None)
//            },

function takes 3 generic arguments but 1 generic argument was supplied
expected 3 generic argumentsrustcClick for full compiler diagnostic
main.rs(109, 27): supplied 1 generic argument
iced_missing.rs(3, 8): function defined here, with 3 generic parameters: `Message`, `Renderer`, `T`
main.rs(109, 72): add missing generic arguments: `, Renderer, T`
encryption_app::iced_missing
pub fn to_elem<'a, Message, Renderer, T>(element: Option<T>) -> Element<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
    T: Into<Element<'a, Message, Theme, Renderer>>,
Message = Element<'_, Message, …, …>

pub fn to_elem<'a, Message, Renderer: iced::advanced::Renderer, T: Into<Element<'a, Message, Theme, Renderer>>>(element: Option<T>) -> Element<'a, Message, Theme, Renderer> {
    struct Void;

    impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Void
    where Renderer: iced::advanced::Renderer
    {
        fn size(&self) -> Size<Length> {
            Size {
                width: Length::Fixed(0.0),
                height: Length::Fixed(0.0),
            }
        }

        fn layout(
            &self,
            _tree: &mut Tree,
            _renderer: &Renderer,
            _limits: &layout::Limits,
        ) -> layout::Node {
            layout::Node::new(Size::ZERO)
        }

        fn draw(
            &self,
            _tree: &Tree,
            _renderer: &mut Renderer,
            _theme: &Theme,
            _style: &renderer::Style,
            _layout: Layout<'_>,
            _cursor: mouse::Cursor,
            _viewport: &Rectangle,
        ) {
        }
    }

    element.map(T::into).unwrap_or_else(|| Element::new(Void))
}
*/


/*
impl<'a, T, Message, Theme, Renderer> From<Option<T>>
    for Element<'a, Message, Theme, Renderer>
where
    T: Into<Self>,
    Renderer: crate::Renderer,
{
    fn from(element: Option<T>) -> Self {
        struct Void;

        impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Void
        where
            Renderer: crate::Renderer,
        {
            fn size(&self) -> Size<Length> {
                Size {
                    width: Length::Fixed(0.0),
                    height: Length::Fixed(0.0),
                }
            }

            fn layout(
                &mut self,
                _tree: &mut Tree,
                _renderer: &Renderer,
                _limits: &layout::Limits,
            ) -> layout::Node {
                layout::Node::new(Size::ZERO)
            }

            fn draw(
                &self,
                _tree: &Tree,
                _renderer: &mut Renderer,
                _theme: &Theme,
                _style: &renderer::Style,
                _layout: Layout<'_>,
                _cursor: mouse::Cursor,
                _viewport: &Rectangle,
            ) {
            }
        }

        element.map(T::into).unwrap_or_else(|| Element::new(Void))
    }
}
*/
