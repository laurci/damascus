// type_of!(MyType) => damascus::spec::Type::schema::<MyType>()
#[macro_export]
macro_rules! type_of {
    ($type:ty) => {
        damascus::spec::Type::schema::<$type>()
    };
}
