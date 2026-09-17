use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TypeId(pub u32);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Type {
    Int,
    Byte,
    Float,
    Bool,
    Char,
    String,
    /// Bootstrap-only read-only command-line argument collection (`string[]`).
    CliArgs,
    Void,
    Dynamic,
    /// A named type before semantic resolution.
    Named(String),
    Struct(TypeId),
    Enum(TypeId),
    Array {
        element: Box<Type>,
        length: usize,
    },
    List(Box<Type>),
    Tuple(Vec<Type>),
}

impl Type {
    /// Aerofyl conversion and coercion rules are not specified, so compatibility
    /// currently means exact equality only. `dynamic` is deliberately not special.
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        self == other
    }
}

impl fmt::Display for Type {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int => formatter.write_str("int"),
            Self::Byte => formatter.write_str("byte"),
            Self::Float => formatter.write_str("float"),
            Self::Bool => formatter.write_str("bool"),
            Self::Char => formatter.write_str("char"),
            Self::String => formatter.write_str("string"),
            Self::CliArgs => formatter.write_str("string[]"),
            Self::Void => formatter.write_str("void"),
            Self::Dynamic => formatter.write_str("dynamic"),
            Self::Named(name) => formatter.write_str(name),
            Self::Struct(id) => write!(formatter, "struct#{}", id.0),
            Self::Enum(id) => write!(formatter, "enum#{}", id.0),
            Self::Array { element, length } => write!(formatter, "{element}[{length}]"),
            Self::List(element) => write!(formatter, "list {element}"),
            Self::Tuple(elements) => {
                formatter.write_str("(")?;
                for (index, element) in elements.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(", ")?;
                    }
                    write!(formatter, "{element}")?;
                }
                formatter.write_str(")")
            }
        }
    }
}
