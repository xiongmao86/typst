use super::*;


/// The arguments passed to a function.
#[derive(Debug, Clone, PartialEq)]
pub struct FuncArgs {
    pub pos: Vec<Spanned<PosArg>>,
    pub key: Vec<Spanned<KeyArg>>,
}

impl FuncArgs {
    /// Create an empty collection of arguments.
    pub fn new() -> FuncArgs {
        FuncArgs {
            pos: vec![],
            key: vec![],
        }
    }

    /// Add a positional argument.
    pub fn add_pos(&mut self, arg: Spanned<PosArg>) {
        self.pos.push(arg);
    }

    /// Add a keyword argument.
    pub fn add_key(&mut self, arg: Spanned<KeyArg>) {
        self.key.push(arg);
    }

    /// Force-extract the first positional argument.
    pub fn get_pos<E: ExpressionKind>(&mut self) -> ParseResult<E> {
        expect(self.get_pos_opt())
    }

    /// Extract the first positional argument.
    pub fn get_pos_opt<E: ExpressionKind>(&mut self) -> ParseResult<Option<E>> {
        Ok(if !self.pos.is_empty() {
            let spanned = self.pos.remove(0);
            Some(E::from_expr(spanned)?)
        } else {
            None
        })
    }

    /// Iterator over positional arguments.
    pub fn pos(&mut self) -> std::vec::IntoIter<Spanned<PosArg>> {
        let vec = std::mem::replace(&mut self.pos, vec![]);
        vec.into_iter()
    }

    /// Force-extract a keyword argument.
    pub fn get_key<E: ExpressionKind>(&mut self, name: &str) -> ParseResult<E> {
        expect(self.get_key_opt(name))
    }

    /// Extract a keyword argument.
    pub fn get_key_opt<E: ExpressionKind>(&mut self, name: &str) -> ParseResult<Option<E>> {
        Ok(if let Some(index) = self.key.iter().position(|arg| arg.v.key.v.0 == name) {
            let value = self.key.swap_remove(index).v.value;
            Some(E::from_expr(value)?)
        } else {
            None
        })
    }

    /// Extract any keyword argument.
    pub fn get_key_next(&mut self) -> Option<Spanned<KeyArg>> {
        self.key.pop()
    }

    /// Iterator over all keyword arguments.
    pub fn keys(&mut self) -> std::vec::IntoIter<Spanned<KeyArg>> {
        let vec = std::mem::replace(&mut self.key, vec![]);
        vec.into_iter()
    }

    /// Clear the argument lists.
    pub fn clear(&mut self) {
        self.pos.clear();
        self.key.clear();
    }

    /// Whether both the positional and keyword argument lists are empty.
    pub fn is_empty(&self) -> bool {
        self.pos.is_empty() && self.key.is_empty()
    }
}

/// Extract the option expression kind from the option or return an error.
fn expect<E: ExpressionKind>(opt: ParseResult<Option<E>>) -> ParseResult<E> {
    match opt {
        Ok(Some(spanned)) => Ok(spanned),
        Ok(None) => error!("expected {}", E::NAME),
        Err(e) => Err(e),
    }
}

/// A positional argument passed to a function.
pub type PosArg = Expression;

/// A keyword argument passed to a function.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyArg {
    pub key: Spanned<Ident>,
    pub value: Spanned<Expression>,
}

/// Either a positional or keyword argument.
#[derive(Debug, Clone, PartialEq)]
pub enum DynArg {
    Pos(Spanned<PosArg>),
    Key(Spanned<KeyArg>),
}

/// An argument or return value.
#[derive(Clone, PartialEq)]
pub enum Expression {
    Ident(Ident),
    Str(String),
    Num(f64),
    Size(Size),
    Bool(bool),
}

impl Display for Expression {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use Expression::*;
        match self {
            Ident(i) => write!(f, "{}", i),
            Str(s) => write!(f, "{:?}", s),
            Num(n) => write!(f, "{}", n),
            Size(s) => write!(f, "{}", s),
            Bool(b) => write!(f, "{}", b),
        }
    }
}

debug_display!(Expression);

pub struct Tuple;
pub struct Object;

/// An identifier.
#[derive(Clone, PartialEq)]
pub struct Ident(pub String);

impl Ident {
    pub fn new<S>(ident: S) -> Option<Ident> where S: AsRef<str> + Into<String> {
        if is_identifier(ident.as_ref()) {
            Some(Ident(ident.into()))
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Display for Ident {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

debug_display!(Ident);

/// Kinds of expressions.
pub trait ExpressionKind: Sized {
    const NAME: &'static str;

    /// Create from expression.
    fn from_expr(expr: Spanned<Expression>) -> ParseResult<Self>;
}

macro_rules! kind {
    ($type:ty, $name:expr, $($patterns:tt)*) => {
        impl ExpressionKind for $type {
            const NAME: &'static str = $name;

            fn from_expr(expr: Spanned<Expression>) -> ParseResult<Self> {
                #[allow(unreachable_patterns)]
                Ok(match expr.v {
                    $($patterns)*,
                    _ => error!("expected {}", Self::NAME),
                })
            }
        }
    };
}

kind!(Expression, "expression", e                         => e);
kind!(Ident,      "identifier", Expression::Ident(ident)  => ident);
kind!(String,     "string",     Expression::Str(string)   => string);
kind!(f64,        "number",     Expression::Num(num)      => num);
kind!(bool,       "boolean",    Expression::Bool(boolean) => boolean);
kind!(Size,       "size",       Expression::Size(size)    => size);
kind!(ScaleSize,  "number or size",
    Expression::Size(size) => ScaleSize::Absolute(size),
    Expression::Num(scale) => ScaleSize::Scaled(scale as f32)
);

impl<T> ExpressionKind for Spanned<T> where T: ExpressionKind {
    const NAME: &'static str = T::NAME;

    fn from_expr(expr: Spanned<Expression>) -> ParseResult<Spanned<T>> {
        let span = expr.span;
        T::from_expr(expr)
            .map(|v| Spanned::new(v, span))
    }
}

impl<T> ExpressionKind for Option<T> where T: ExpressionKind {
    const NAME: &'static str = T::NAME;

    fn from_expr(expr: Spanned<Expression>) -> ParseResult<Option<T>> {
        if let Expression::Ident(ident) = &expr.v {
            match ident.as_str() {
                "default" | "none" => return Ok(None),
                _ => {},
            }
        }

        T::from_expr(expr).map(|v| Some(v))
    }
}