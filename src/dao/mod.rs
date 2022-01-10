trait Dao {
    fn create(&self) -> Result<(), DaoError>;
    fn read(&self) -> Result<(), DaoError>;
    fn update(&self) -> Result<(), DaoError>;
    fn delete(&self) -> Result<(), DaoError>;
}

struct DaoError {
    details: String,
}
