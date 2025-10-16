#[cfg_attr(not(test), expect(unused, reason = "Not used outside of testing"))]
pub trait PrettyPrinterWrapper {
    fn display_as_debug(&'_ self) -> DisplayAsDebugWrapper<'_, Self>
    where
        Self: std::fmt::Display,
    {
        DisplayAsDebugWrapper::<Self> { inner: self }
    }

    fn pretty_print<F>(&'_ self, format: F) -> PrettyPrinter<'_, Self, F>
    where
        F: Fn(&Self, &mut std::fmt::Formatter<'_>) -> std::fmt::Result,
    {
        PrettyPrinter {
            inner: self,
            formatter: format,
        }
    }
}

impl<T> PrettyPrinterWrapper for T {}

pub struct DisplayAsDebugWrapper<'t, T>
where
    T: std::fmt::Display + ?Sized,
{
    inner: &'t T,
}

impl<T> std::fmt::Debug for DisplayAsDebugWrapper<'_, T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, f)
    }
}

pub struct PrettyPrinter<'t, T, F>
where
    F: Fn(&'t T, &mut std::fmt::Formatter<'_>) -> std::fmt::Result,
    T: ?Sized,
{
    formatter: F,
    inner: &'t T,
}

impl<'t, T, F> std::fmt::Debug for PrettyPrinter<'t, T, F>
where
    F: Fn(&'t T, &mut std::fmt::Formatter<'_>) -> std::fmt::Result,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (self.formatter)(self.inner, f)
    }
}

impl<'t, T, F> std::fmt::Display for PrettyPrinter<'t, T, F>
where
    F: Fn(&'t T, &mut std::fmt::Formatter<'_>) -> std::fmt::Result,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (self.formatter)(self.inner, f)
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Formatter;

    use crate::traits::display_as_debug::PrettyPrinterWrapper as _;

    #[test]
    fn display_as_debug() {
        struct OnlyDisplay {}

        impl std::fmt::Display for OnlyDisplay {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("only_display")
            }
        }

        let only_display = (OnlyDisplay {}).display_as_debug();

        assert_eq!("only_display", format!("{:?}", only_display));
    }

    #[test]
    fn custom_formatter() {
        fn hungry_pinter(hungry: &Hungry, f: &mut Formatter<'_>) -> std::fmt::Result {
            if hungry.is_hungry {
                write!(f, "I'm hungry")
            } else {
                write!(f, "I'm not hungry")
            }
        }

        struct Hungry {
            is_hungry: bool,
        }

        let custom_formatter = (Hungry { is_hungry: true }).pretty_print(hungry_pinter);

        assert_eq!("I'm hungry", format!("{:?}", &custom_formatter));
        assert_eq!("I'm hungry", format!("{}", &custom_formatter));
    }
}
