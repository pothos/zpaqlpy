
/// command line options and their default values (please keep in sync with option parsing main.rs)

pub struct Options {
    pub emit_ir: bool,
    pub suppress_hcomp: bool,
    pub suppress_pcomp: bool,
    pub disable_comp: bool,
    pub extern_tokenizer: bool,
    pub comments: bool,
    pub stacksize: u32,
    pub disable_optim: bool,
    pub fixed_global_access: bool,
    pub ignore_errors: bool,
    pub pc_as_comment: bool,
    pub no_post_zpaql: bool,

    pub temp_debug_cfg: bool,
}

impl Options {
    pub fn new() -> Options {
        Options{ // anyway overwritten in main.rs, but try to keep in sync
            emit_ir: false,
            suppress_hcomp: false,
            suppress_pcomp: false,
            disable_comp: false,
            extern_tokenizer: false,
            comments: true,
            disable_optim: false,
            fixed_global_access: false,
            ignore_errors: false,
            pc_as_comment: true,
            temp_debug_cfg: true,
            no_post_zpaql: false,
            stacksize: 1048576,  // 1 MB
        }
    }
}
