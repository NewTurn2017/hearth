//! Hearth pure logic layer — schema, migrations, domain modules,
//! audit log, search, views. No Tauri dependency.

pub mod audit;
pub mod db;
pub mod models;
pub mod projects;

#[cfg(test)]
mod smoke_tests {
    #[test]
    fn core_compiles() {
        assert_eq!(1 + 1, 2);
    }
}
