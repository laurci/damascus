// type_of!(MyType) => damascus::spec::Type::schema::<MyType>()
#[macro_export]
macro_rules! type_of {
    ($type:ty) => {
        damascus::spec::Type::schema::<$type>()
    };
}

#[macro_export]
macro_rules! type_of_tuple {
    ($($type:ty),*) => {
        damascus::spec::Type::tuple(vec![$(damascus::type_of!($type)),*])
    };
}

