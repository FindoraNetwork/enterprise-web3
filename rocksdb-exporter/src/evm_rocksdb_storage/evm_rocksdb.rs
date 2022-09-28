use {
    rocksdb::{DBIterator, Options, ReadOptions, DB},
    ruc::*,
};

pub struct RocksDB {
    db: DB,
}

impl RocksDB {
    fn default_db_opts() -> Options {
        let mut opts = Options::default();
        opts.set_allow_mmap_reads(true);
        opts
    }
    pub fn open(path: &str) -> Result<Self> {
        let opts = Self::default_db_opts();
        let cf_names = DB::list_cf(&opts, path).c(d!())?;
        let db = DB::open_cf_for_read_only(&opts, path, &cf_names, false).c(d!())?;
        Ok(Self { db })
    }
    pub fn iterate(
        &self,
        lower: &[u8],
        upper: &[u8],
        asc: bool,
        cf_name: &str,
    ) -> Result<DBIterator> {
        let mut readopts = ReadOptions::default();
        if !(lower.is_empty() || upper.is_empty()) {
            readopts.set_iterate_lower_bound(lower.to_vec());
            readopts.set_iterate_upper_bound(upper.to_vec());
        }
        let cf = self.db.cf_handle(cf_name).c(d!())?;
        let iter = if asc {
            self.db
                .iterator_cf_opt(cf, readopts, rocksdb::IteratorMode::Start)
        } else {
            self.db
                .iterator_cf_opt(cf, readopts, rocksdb::IteratorMode::End)
        };
        Ok(iter)
    }
    pub fn get(&self, cf_name: &str, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let cf = self.db.cf_handle(cf_name).c(d!())?;
        self.db.get_cf(cf, key).c(d!())
    }
}
