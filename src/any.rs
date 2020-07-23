//! Helper module which defines the `Any` trait to to allow dynamic value handling.

use crate::fn_native::{FnPtr, SendSync};
use crate::parser::{ImmutableString, INT};
use crate::r#unsafe::{unsafe_cast_box, unsafe_try_cast};

#[cfg(not(feature = "no_float"))]
use crate::parser::FLOAT;

#[cfg(not(feature = "no_index"))]
use crate::engine::Array;

#[cfg(not(feature = "no_object"))]
use crate::engine::Map;

use crate::stdlib::{
    any::{type_name, Any, TypeId},
    boxed::Box,
    collections::HashMap,
    fmt,
    string::String,
    vec::Vec,
};

#[cfg(not(feature = "no_std"))]
#[cfg(not(target_arch = "wasm32"))]
use crate::stdlib::time::Instant;

#[cfg(not(feature = "no_std"))]
#[cfg(target_arch = "wasm32")]
use instant::Instant;

/// Trait to represent any type.
///
/// Currently, `Variant` is not `Send` nor `Sync`, so it can practically be any type.
/// Turn on the `sync` feature to restrict it to only types that implement `Send + Sync`.
#[cfg(not(feature = "sync"))]
pub trait Variant: Any {
    /// Convert this `Variant` trait object to `&dyn Any`.
    fn as_any(&self) -> &dyn Any;

    /// Convert this `Variant` trait object to `&mut dyn Any`.
    fn as_mut_any(&mut self) -> &mut dyn Any;

    /// Convert this `Variant` trait object to an `Any` trait object.
    fn as_box_any(self: Box<Self>) -> Box<dyn Any>;

    /// Get the name of this type.
    fn type_name(&self) -> &'static str;

    /// Convert into `Dynamic`.
    fn into_dynamic(self) -> Dynamic;

    /// Clone into `Dynamic`.
    fn clone_into_dynamic(&self) -> Dynamic;

    /// This trait may only be implemented by `rhai`.
    #[doc(hidden)]
    fn _closed(&self) -> _Private;
}

/// Trait to represent any type.
///
/// `From<_>` is implemented for `i64` (`i32` if `only_i32`), `f64` (if not `no_float`),
/// `bool`, `String`, `char`, `Vec<T>` (into `Array`) and `HashMap<String, T>` (into `Map`).
#[cfg(feature = "sync")]
pub trait Variant: Any + Send + Sync {
    /// Convert this `Variant` trait object to `&dyn Any`.
    fn as_any(&self) -> &dyn Any;

    /// Convert this `Variant` trait object to `&mut dyn Any`.
    fn as_mut_any(&mut self) -> &mut dyn Any;

    /// Convert this `Variant` trait object to an `Any` trait object.
    fn as_box_any(self: Box<Self>) -> Box<dyn Any>;

    /// Get the name of this type.
    fn type_name(&self) -> &'static str;

    /// Convert into `Dynamic`.
    fn into_dynamic(self) -> Dynamic;

    /// Clone into `Dynamic`.
    fn clone_into_dynamic(&self) -> Dynamic;

    /// This trait may only be implemented by `rhai`.
    #[doc(hidden)]
    fn _closed(&self) -> _Private;
}

impl<T: Any + Clone + SendSync> Variant for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
    fn as_box_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
    fn type_name(&self) -> &'static str {
        type_name::<T>()
    }
    fn into_dynamic(self) -> Dynamic {
        Dynamic::from(self)
    }
    fn clone_into_dynamic(&self) -> Dynamic {
        Dynamic::from(self.clone())
    }
    fn _closed(&self) -> _Private {
        _Private
    }
}

impl dyn Variant {
    /// Is this `Variant` a specific type?
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id()
    }
}

/// Dynamic type containing any value.
pub struct Dynamic(pub(crate) Union);

/// Internal `Dynamic` representation.
///
/// Most variants are boxed to reduce the size.
pub enum Union {
    Unit(()),
    Bool(bool),
    Str(ImmutableString),
    Char(char),
    Int(INT),
    #[cfg(not(feature = "no_float"))]
    Float(FLOAT),
    #[cfg(not(feature = "no_index"))]
    Array(Box<Array>),
    #[cfg(not(feature = "no_object"))]
    Map(Box<Map>),
    FnPtr(Box<FnPtr>),
    Variant(Box<Box<dyn Variant>>),
}

impl Dynamic {
    /// Does this `Dynamic` hold a variant data type
    /// instead of one of the support system primitive types?
    pub fn is_variant(&self) -> bool {
        match self.0 {
            Union::Variant(_) => true,
            _ => false,
        }
    }

    /// Is the value held by this `Dynamic` a particular type?
    pub fn is<T: Variant + Clone>(&self) -> bool {
        self.type_id() == TypeId::of::<T>()
            || match self.0 {
                Union::Str(_) => TypeId::of::<String>() == TypeId::of::<T>(),
                _ => false,
            }
    }

    /// Get the TypeId of the value held by this `Dynamic`.
    pub fn type_id(&self) -> TypeId {
        match &self.0 {
            Union::Unit(_) => TypeId::of::<()>(),
            Union::Bool(_) => TypeId::of::<bool>(),
            Union::Str(_) => TypeId::of::<ImmutableString>(),
            Union::Char(_) => TypeId::of::<char>(),
            Union::Int(_) => TypeId::of::<INT>(),
            #[cfg(not(feature = "no_float"))]
            Union::Float(_) => TypeId::of::<FLOAT>(),
            #[cfg(not(feature = "no_index"))]
            Union::Array(_) => TypeId::of::<Array>(),
            #[cfg(not(feature = "no_object"))]
            Union::Map(_) => TypeId::of::<Map>(),
            Union::FnPtr(_) => TypeId::of::<FnPtr>(),
            Union::Variant(value) => (***value).type_id(),
        }
    }

    /// Get the name of the type of the value held by this `Dynamic`.
    pub fn type_name(&self) -> &'static str {
        match &self.0 {
            Union::Unit(_) => "()",
            Union::Bool(_) => "bool",
            Union::Str(_) => "string",
            Union::Char(_) => "char",
            Union::Int(_) => type_name::<INT>(),
            #[cfg(not(feature = "no_float"))]
            Union::Float(_) => type_name::<FLOAT>(),
            #[cfg(not(feature = "no_index"))]
            Union::Array(_) => "array",
            #[cfg(not(feature = "no_object"))]
            Union::Map(_) => "map",
            Union::FnPtr(_) => "Fn",

            #[cfg(not(feature = "no_std"))]
            Union::Variant(value) if value.is::<Instant>() => "timestamp",
            Union::Variant(value) => (***value).type_name(),
        }
    }
}

/// Map the name of a standard type into a friendly form.
#[inline]
pub(crate) fn map_std_type_name(name: &str) -> &str {
    if name == type_name::<String>() {
        "string"
    } else if name == type_name::<ImmutableString>() {
        "string"
    } else if name == type_name::<&str>() {
        "string"
    } else if name == type_name::<FnPtr>() {
        "Fn"
    } else {
        #[cfg(not(feature = "no_std"))]
        if name == type_name::<Instant>() {
            return "timestamp";
        }
        #[cfg(not(feature = "no_index"))]
        if name == type_name::<Array>() {
            return "array";
        }
        #[cfg(not(feature = "no_object"))]
        if name == type_name::<Map>() {
            return "map";
        }

        name
    }
}

impl fmt::Display for Dynamic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Union::Unit(_) => write!(f, ""),
            Union::Bool(value) => fmt::Display::fmt(value, f),
            Union::Str(value) => fmt::Display::fmt(value, f),
            Union::Char(value) => fmt::Display::fmt(value, f),
            Union::Int(value) => fmt::Display::fmt(value, f),
            #[cfg(not(feature = "no_float"))]
            Union::Float(value) => fmt::Display::fmt(value, f),
            #[cfg(not(feature = "no_index"))]
            Union::Array(value) => fmt::Debug::fmt(value, f),
            #[cfg(not(feature = "no_object"))]
            Union::Map(value) => {
                f.write_str("#")?;
                fmt::Debug::fmt(value, f)
            }
            Union::FnPtr(value) => fmt::Display::fmt(value, f),

            #[cfg(not(feature = "no_std"))]
            Union::Variant(value) if value.is::<Instant>() => write!(f, "<timestamp>"),
            Union::Variant(value) => write!(f, "{}", (*value).type_name()),
        }
    }
}

impl fmt::Debug for Dynamic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Union::Unit(value) => fmt::Debug::fmt(value, f),
            Union::Bool(value) => fmt::Debug::fmt(value, f),
            Union::Str(value) => fmt::Debug::fmt(value, f),
            Union::Char(value) => fmt::Debug::fmt(value, f),
            Union::Int(value) => fmt::Debug::fmt(value, f),
            #[cfg(not(feature = "no_float"))]
            Union::Float(value) => fmt::Debug::fmt(value, f),
            #[cfg(not(feature = "no_index"))]
            Union::Array(value) => fmt::Debug::fmt(value, f),
            #[cfg(not(feature = "no_object"))]
            Union::Map(value) => {
                f.write_str("#")?;
                fmt::Debug::fmt(value, f)
            }
            Union::FnPtr(value) => fmt::Debug::fmt(value, f),

            #[cfg(not(feature = "no_std"))]
            Union::Variant(value) if value.is::<Instant>() => write!(f, "<timestamp>"),
            Union::Variant(value) => write!(f, "{}", (*value).type_name()),
        }
    }
}

impl Clone for Dynamic {
    fn clone(&self) -> Self {
        match self.0 {
            Union::Unit(value) => Self(Union::Unit(value)),
            Union::Bool(value) => Self(Union::Bool(value)),
            Union::Str(ref value) => Self(Union::Str(value.clone())),
            Union::Char(value) => Self(Union::Char(value)),
            Union::Int(value) => Self(Union::Int(value)),
            #[cfg(not(feature = "no_float"))]
            Union::Float(value) => Self(Union::Float(value)),
            #[cfg(not(feature = "no_index"))]
            Union::Array(ref value) => Self(Union::Array(value.clone())),
            #[cfg(not(feature = "no_object"))]
            Union::Map(ref value) => Self(Union::Map(value.clone())),
            Union::FnPtr(ref value) => Self(Union::FnPtr(value.clone())),
            Union::Variant(ref value) => (***value).clone_into_dynamic(),
        }
    }
}

impl Default for Dynamic {
    fn default() -> Self {
        Self(Union::Unit(()))
    }
}

impl Dynamic {
    /// Create a `Dynamic` from any type.  A `Dynamic` value is simply returned as is.
    ///
    /// # Safety
    ///
    /// This type uses some unsafe code, mainly for type casting.
    ///
    /// # Notes
    ///
    /// Beware that you need to pass in an `Array` type for it to be recognized as an `Array`.
    /// A `Vec<T>` does not get automatically converted to an `Array`, but will be a generic
    /// restricted trait object instead, because `Vec<T>` is not a supported standard type.
    ///
    /// Similarly, passing in a `HashMap<String, T>` will not get a `Map` but a trait object.
    ///
    /// # Examples
    ///
    /// ```
    /// use rhai::Dynamic;
    ///
    /// let result = Dynamic::from(42_i64);
    /// assert_eq!(result.type_name(), "i64");
    /// assert_eq!(result.to_string(), "42");
    ///
    /// let result = Dynamic::from("hello".to_string());
    /// assert_eq!(result.type_name(), "string");
    /// assert_eq!(result.to_string(), "hello");
    ///
    /// let new_result = Dynamic::from(result);
    /// assert_eq!(new_result.type_name(), "string");
    /// assert_eq!(new_result.to_string(), "hello");
    /// ```
    #[inline(always)]
    pub fn from<T: Variant + Clone>(value: T) -> Self {
        let type_id = TypeId::of::<T>();

        if type_id == TypeId::of::<INT>() {
            return <dyn Any>::downcast_ref::<INT>(&value)
                .unwrap()
                .clone()
                .into();
        }
        #[cfg(not(feature = "no_float"))]
        if type_id == TypeId::of::<FLOAT>() {
            return <dyn Any>::downcast_ref::<FLOAT>(&value)
                .unwrap()
                .clone()
                .into();
        }
        if type_id == TypeId::of::<bool>() {
            return <dyn Any>::downcast_ref::<bool>(&value)
                .unwrap()
                .clone()
                .into();
        }
        if type_id == TypeId::of::<char>() {
            return <dyn Any>::downcast_ref::<char>(&value)
                .unwrap()
                .clone()
                .into();
        }
        if type_id == TypeId::of::<ImmutableString>() {
            return <dyn Any>::downcast_ref::<ImmutableString>(&value)
                .unwrap()
                .clone()
                .into();
        }
        if type_id == TypeId::of::<()>() {
            return ().into();
        }

        let mut boxed = Box::new(value);

        boxed = match unsafe_cast_box::<_, String>(boxed) {
            Ok(s) => return (*s).into(),
            Err(val) => val,
        };
        #[cfg(not(feature = "no_index"))]
        {
            boxed = match unsafe_cast_box::<_, Array>(boxed) {
                Ok(array) => return (*array).into(),
                Err(val) => val,
            };
        }

        #[cfg(not(feature = "no_object"))]
        {
            boxed = match unsafe_cast_box::<_, Map>(boxed) {
                Ok(map) => return (*map).into(),
                Err(val) => val,
            }
        }

        boxed = match unsafe_cast_box::<_, Dynamic>(boxed) {
            Ok(d) => return *d,
            Err(val) => val,
        };

        Self(Union::Variant(Box::new(boxed)))
    }

    /// Get a copy of the `Dynamic` value as a specific type.
    /// Casting to a `Dynamic` just returns as is.
    ///
    /// Returns an error with the name of the value's actual type when the cast fails.
    ///
    /// # Example
    ///
    /// ```
    /// use rhai::Dynamic;
    ///
    /// let x = Dynamic::from(42_u32);
    ///
    /// assert_eq!(x.try_cast::<u32>().unwrap(), 42);
    /// ```
    #[inline(always)]
    pub fn try_cast<T: Variant>(self) -> Option<T> {
        let type_id = TypeId::of::<T>();

        if type_id == TypeId::of::<INT>() {
            return match self.0 {
                Union::Int(value) => unsafe_try_cast(value),
                _ => None,
            };
        }
        #[cfg(not(feature = "no_float"))]
        if type_id == TypeId::of::<FLOAT>() {
            return match self.0 {
                Union::Float(value) => unsafe_try_cast(value),
                _ => None,
            };
        }
        if type_id == TypeId::of::<bool>() {
            return match self.0 {
                Union::Bool(value) => unsafe_try_cast(value),
                _ => None,
            };
        }
        if type_id == TypeId::of::<ImmutableString>() {
            return match self.0 {
                Union::Str(value) => unsafe_try_cast(value),
                _ => None,
            };
        }
        if type_id == TypeId::of::<String>() {
            return match self.0 {
                Union::Str(value) => unsafe_try_cast(value.into_owned()),
                _ => None,
            };
        }
        if type_id == TypeId::of::<char>() {
            return match self.0 {
                Union::Char(value) => unsafe_try_cast(value),
                _ => None,
            };
        }
        #[cfg(not(feature = "no_index"))]
        if type_id == TypeId::of::<Array>() {
            return match self.0 {
                Union::Array(value) => unsafe_cast_box::<_, T>(value).ok().map(|v| *v),
                _ => None,
            };
        }
        #[cfg(not(feature = "no_object"))]
        if type_id == TypeId::of::<Map>() {
            return match self.0 {
                Union::Map(value) => unsafe_cast_box::<_, T>(value).ok().map(|v| *v),
                _ => None,
            };
        }
        if type_id == TypeId::of::<FnPtr>() {
            return match self.0 {
                Union::FnPtr(value) => unsafe_cast_box::<_, T>(value).ok().map(|v| *v),
                _ => None,
            };
        }
        if type_id == TypeId::of::<()>() {
            return match self.0 {
                Union::Unit(value) => unsafe_try_cast(value),
                _ => None,
            };
        }
        if type_id == TypeId::of::<Dynamic>() {
            return unsafe_cast_box::<_, T>(Box::new(self)).ok().map(|v| *v);
        }

        match self.0 {
            Union::Variant(value) => (*value).as_box_any().downcast().map(|x| *x).ok(),
            _ => None,
        }
    }

    /// Get a copy of the `Dynamic` value as a specific type.
    /// Casting to a `Dynamic` just returns as is.
    ///
    /// # Panics
    ///
    /// Panics if the cast fails (e.g. the type of the actual value is not the same as the specified type).
    ///
    /// # Example
    ///
    /// ```
    /// use rhai::Dynamic;
    ///
    /// let x = Dynamic::from(42_u32);
    ///
    /// assert_eq!(x.cast::<u32>(), 42);
    /// ```
    #[inline(always)]
    pub fn cast<T: Variant + Clone>(self) -> T {
        self.try_cast::<T>().unwrap()
    }

    /// Get a reference of a specific type to the `Dynamic`.
    /// Casting to `Dynamic` just returns a reference to it.
    /// Returns `None` if the cast fails.
    #[inline(always)]
    pub fn downcast_ref<T: Variant + Clone>(&self) -> Option<&T> {
        let type_id = TypeId::of::<T>();

        if type_id == TypeId::of::<INT>() {
            return match &self.0 {
                Union::Int(value) => <dyn Any>::downcast_ref::<T>(value),
                _ => None,
            };
        }
        #[cfg(not(feature = "no_float"))]
        if type_id == TypeId::of::<FLOAT>() {
            return match &self.0 {
                Union::Float(value) => <dyn Any>::downcast_ref::<T>(value),
                _ => None,
            };
        }
        if type_id == TypeId::of::<bool>() {
            return match &self.0 {
                Union::Bool(value) => <dyn Any>::downcast_ref::<T>(value),
                _ => None,
            };
        }
        if type_id == TypeId::of::<ImmutableString>() {
            return match &self.0 {
                Union::Str(value) => <dyn Any>::downcast_ref::<T>(value),
                _ => None,
            };
        }
        if type_id == TypeId::of::<String>() {
            return match &self.0 {
                Union::Str(value) => <dyn Any>::downcast_ref::<T>(value.as_ref()),
                _ => None,
            };
        }
        if type_id == TypeId::of::<char>() {
            return match &self.0 {
                Union::Char(value) => <dyn Any>::downcast_ref::<T>(value),
                _ => None,
            };
        }
        #[cfg(not(feature = "no_index"))]
        if type_id == TypeId::of::<Array>() {
            return match &self.0 {
                Union::Array(value) => <dyn Any>::downcast_ref::<T>(value.as_ref()),
                _ => None,
            };
        }
        #[cfg(not(feature = "no_object"))]
        if type_id == TypeId::of::<Map>() {
            return match &self.0 {
                Union::Map(value) => <dyn Any>::downcast_ref::<T>(value.as_ref()),
                _ => None,
            };
        }
        if type_id == TypeId::of::<FnPtr>() {
            return match &self.0 {
                Union::FnPtr(value) => <dyn Any>::downcast_ref::<T>(value.as_ref()),
                _ => None,
            };
        }
        if type_id == TypeId::of::<()>() {
            return match &self.0 {
                Union::Unit(value) => <dyn Any>::downcast_ref::<T>(value),
                _ => None,
            };
        }
        if type_id == TypeId::of::<Dynamic>() {
            return <dyn Any>::downcast_ref::<T>(self);
        }

        match &self.0 {
            Union::Variant(value) => value.as_ref().as_ref().as_any().downcast_ref::<T>(),
            _ => None,
        }
    }

    /// Get a mutable reference of a specific type to the `Dynamic`.
    /// Casting to `Dynamic` just returns a mutable reference to it.
    /// Returns `None` if the cast fails.
    #[inline(always)]
    pub fn downcast_mut<T: Variant + Clone>(&mut self) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();

        if type_id == TypeId::of::<INT>() {
            return match &mut self.0 {
                Union::Int(value) => <dyn Any>::downcast_mut::<T>(value),
                _ => None,
            };
        }
        #[cfg(not(feature = "no_float"))]
        if type_id == TypeId::of::<FLOAT>() {
            return match &mut self.0 {
                Union::Float(value) => <dyn Any>::downcast_mut::<T>(value),
                _ => None,
            };
        }
        if type_id == TypeId::of::<bool>() {
            return match &mut self.0 {
                Union::Bool(value) => <dyn Any>::downcast_mut::<T>(value),
                _ => None,
            };
        }
        if type_id == TypeId::of::<ImmutableString>() {
            return match &mut self.0 {
                Union::Str(value) => <dyn Any>::downcast_mut::<T>(value),
                _ => None,
            };
        }
        if type_id == TypeId::of::<char>() {
            return match &mut self.0 {
                Union::Char(value) => <dyn Any>::downcast_mut::<T>(value),
                _ => None,
            };
        }
        #[cfg(not(feature = "no_index"))]
        if type_id == TypeId::of::<Array>() {
            return match &mut self.0 {
                Union::Array(value) => <dyn Any>::downcast_mut::<T>(value.as_mut()),
                _ => None,
            };
        }
        #[cfg(not(feature = "no_object"))]
        if type_id == TypeId::of::<Map>() {
            return match &mut self.0 {
                Union::Map(value) => <dyn Any>::downcast_mut::<T>(value.as_mut()),
                _ => None,
            };
        }
        if type_id == TypeId::of::<FnPtr>() {
            return match &mut self.0 {
                Union::FnPtr(value) => <dyn Any>::downcast_mut::<T>(value.as_mut()),
                _ => None,
            };
        }
        if type_id == TypeId::of::<()>() {
            return match &mut self.0 {
                Union::Unit(value) => <dyn Any>::downcast_mut::<T>(value),
                _ => None,
            };
        }
        if type_id == TypeId::of::<Dynamic>() {
            return <dyn Any>::downcast_mut::<T>(self);
        }

        match &mut self.0 {
            Union::Variant(value) => value.as_mut().as_mut_any().downcast_mut::<T>(),
            _ => None,
        }
    }

    /// Cast the `Dynamic` as the system integer type `INT` and return it.
    /// Returns the name of the actual type if the cast fails.
    pub fn as_int(&self) -> Result<INT, &'static str> {
        match self.0 {
            Union::Int(n) => Ok(n),
            _ => Err(self.type_name()),
        }
    }

    /// Cast the `Dynamic` as the system floating-point type `FLOAT` and return it.
    /// Returns the name of the actual type if the cast fails.
    #[cfg(not(feature = "no_float"))]
    pub fn as_float(&self) -> Result<FLOAT, &'static str> {
        match self.0 {
            Union::Float(n) => Ok(n),
            _ => Err(self.type_name()),
        }
    }

    /// Cast the `Dynamic` as a `bool` and return it.
    /// Returns the name of the actual type if the cast fails.
    pub fn as_bool(&self) -> Result<bool, &'static str> {
        match self.0 {
            Union::Bool(b) => Ok(b),
            _ => Err(self.type_name()),
        }
    }

    /// Cast the `Dynamic` as a `char` and return it.
    /// Returns the name of the actual type if the cast fails.
    pub fn as_char(&self) -> Result<char, &'static str> {
        match self.0 {
            Union::Char(n) => Ok(n),
            _ => Err(self.type_name()),
        }
    }

    /// Cast the `Dynamic` as a string and return the string slice.
    /// Returns the name of the actual type if the cast fails.
    pub fn as_str(&self) -> Result<&str, &'static str> {
        match &self.0 {
            Union::Str(s) => Ok(s),
            Union::FnPtr(f) => Ok(f.fn_name()),
            _ => Err(self.type_name()),
        }
    }

    /// Convert the `Dynamic` into `String` and return it.
    /// Returns the name of the actual type if the cast fails.
    pub fn take_string(self) -> Result<String, &'static str> {
        self.take_immutable_string()
            .map(ImmutableString::into_owned)
    }

    /// Convert the `Dynamic` into `ImmutableString` and return it.
    /// Returns the name of the actual type if the cast fails.
    pub(crate) fn take_immutable_string(self) -> Result<ImmutableString, &'static str> {
        match self.0 {
            Union::Str(s) => Ok(s),
            Union::FnPtr(f) => Ok(f.take_data().0),
            _ => Err(self.type_name()),
        }
    }
}

impl From<()> for Dynamic {
    fn from(value: ()) -> Self {
        Self(Union::Unit(value))
    }
}
impl From<bool> for Dynamic {
    fn from(value: bool) -> Self {
        Self(Union::Bool(value))
    }
}
impl From<INT> for Dynamic {
    fn from(value: INT) -> Self {
        Self(Union::Int(value))
    }
}
#[cfg(not(feature = "no_float"))]
impl From<FLOAT> for Dynamic {
    fn from(value: FLOAT) -> Self {
        Self(Union::Float(value))
    }
}
impl From<char> for Dynamic {
    fn from(value: char) -> Self {
        Self(Union::Char(value))
    }
}
impl<S: Into<ImmutableString>> From<S> for Dynamic {
    fn from(value: S) -> Self {
        Self(Union::Str(value.into()))
    }
}
#[cfg(not(feature = "no_index"))]
impl<T: Variant + Clone> From<Vec<T>> for Dynamic {
    fn from(value: Vec<T>) -> Self {
        Self(Union::Array(Box::new(
            value.into_iter().map(Dynamic::from).collect(),
        )))
    }
}
#[cfg(not(feature = "no_index"))]
impl<T: Variant + Clone> From<&[T]> for Dynamic {
    fn from(value: &[T]) -> Self {
        Self(Union::Array(Box::new(
            value.iter().cloned().map(Dynamic::from).collect(),
        )))
    }
}
#[cfg(not(feature = "no_object"))]
impl<K: Into<ImmutableString>, T: Variant + Clone> From<HashMap<K, T>> for Dynamic {
    fn from(value: HashMap<K, T>) -> Self {
        Self(Union::Map(Box::new(
            value
                .into_iter()
                .map(|(k, v)| (k.into(), Dynamic::from(v)))
                .collect(),
        )))
    }
}
impl From<FnPtr> for Dynamic {
    fn from(value: FnPtr) -> Self {
        Box::new(value).into()
    }
}
impl From<Box<FnPtr>> for Dynamic {
    fn from(value: Box<FnPtr>) -> Self {
        Self(Union::FnPtr(value))
    }
}

/// Private type which ensures that `rhai::Any` and `rhai::AnyExt` can only
/// be implemented by this crate.
#[doc(hidden)]
pub struct _Private;
