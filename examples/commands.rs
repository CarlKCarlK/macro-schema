// examples/commands.rs: run with `cargo run --example commands`.
//
// A `commands!` declaration turns a named list of commands into an enum with a
// parser, argument checking, help text, and a usage counter per command.

// ----- Library author: defines the `commands!` declaration macro -----

#[doc(hidden)]
pub use macro_schema::expand as __macro_schema_expand;

pub mod cli {
    macro_schema::define! {
        /// Declares the commands of a command-line tool as an enum.
        ///
        /// Each member becomes a variant. Its command word is the variant name in
        /// `snake_case`.
        pub commands {
            /// Text shown at the top of the help output.
            about: expr,
            /// Each member is one command.
            members 1.. {
                /// One-line description shown in the help output.
                summary: expr,
                /// Whether the command is listed in the help output.
                listed: expr = true,
                /// Accepted number of arguments; without it, the command takes none.
                args?: {
                    /// Fewest arguments accepted.
                    min: expr,
                    /// Most arguments accepted.
                    max: expr,
                },
            }
        }

        generate {
            #[derive(Clone, Copy, Debug, PartialEq, Eq)]
            $decl.attrs
            #[doc = $decl.doc]
            $decl.vis enum $decl.name {
                $for command in $decl.members {
                    $command.attrs
                    #[doc = $command.doc]
                    $command.name,
                }
            }

            /// The word that `parse` didn't recognize.
            #[derive(Debug, PartialEq, Eq)]
            $decl.vis struct $ident($decl.name, ParseError)(pub String);

            $for command in $decl.members {
                static $upper($decl.name, _, $command.name, _USES): ::std::sync::atomic::AtomicUsize =
                    ::std::sync::atomic::AtomicUsize::new(0);
            }

            impl $decl.name {
                /// Every command, in declaration order.
                pub const ALL: &'static [Self] = &[$for command in $decl.members { Self::$command.name, }];

                /// Text shown at the top of the help output.
                pub const ABOUT: &'static str = $decl.about;

                /// The word that selects this command.
                pub fn word(self) -> &'static str {
                    match self {
                        $for command in $decl.members {
                            Self::$command.name => stringify!($snake($command.name)),
                        }
                    }
                }

                /// Finds the command for a word.
                pub fn parse(word: &str) -> Result<Self, $ident($decl.name, ParseError)> {
                    Self::ALL
                        .iter()
                        .copied()
                        .find(|command| command.word() == word)
                        .ok_or_else(|| $ident($decl.name, ParseError)(word.to_owned()))
                }

                /// Whether the command accepts `count` arguments.
                pub fn accepts(self, count: usize) -> bool {
                    match self {
                        $for command in $decl.members {
                            Self::$command.name => $if let Some(args) = $command.args {
                                ($args.min..=$args.max).contains(&count)
                            } else {
                                count == 0
                            },
                        }
                    }
                }

                /// Records one use of the command and returns its new use count.
                pub fn record_use(self) -> usize {
                    let uses = match self {
                        $for command in $decl.members {
                            Self::$command.name => &$upper($decl.name, _, $command.name, _USES),
                        }
                    };
                    uses.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed) + 1
                }

                /// Help text: `ABOUT`, then one line per listed command.
                pub fn help() -> String {
                    let mut help = String::from(Self::ABOUT);
                    $for command in $decl.members {
                        if $command.listed {
                            help.push_str(&format!(
                                "\n  {:<8}{}",
                                stringify!($snake($command.name)),
                                $command.summary,
                            ));
                        }
                    }
                    help
                }
            }
        }
    }
}

// ----- Library user: declares the commands of a small file tool -----

cli::commands! {
    #[derive(Hash)]
    pub FileTool {
        about: "A small file tool.",

        List { summary: "List files" },
        Copy { summary: "Copy a file", args: { min: 2, max: 2 } },
        /// Removes files. This written doc replaces the generated one.
        Remove { summary: "Remove files", args: { min: 1, max: usize::MAX } },
        DebugDump { summary: "Dump internal state", listed: false },
    }
}

fn main() {
    println!("{}", FileTool::help());

    let command = FileTool::parse("copy").expect("`copy` is a command");
    assert_eq!(command, FileTool::Copy);
    assert!(command.accepts(2));
    assert!(!command.accepts(1));
    assert!(FileTool::List.accepts(0));
    assert_eq!(FileTool::DebugDump.word(), "debug_dump");
    assert_eq!(
        FileTool::parse("frob"),
        Err(FileToolParseError("frob".to_owned()))
    );

    assert_eq!(command.record_use(), 1);
    assert_eq!(command.record_use(), 2);
    assert_eq!(FileTool::List.record_use(), 1);
    // The counter behind `Copy` is `$upper($decl.name, _, $command.name, _USES)`.
    assert_eq!(
        FILE_TOOL_COPY_USES.load(std::sync::atomic::Ordering::Relaxed),
        2
    );
}
