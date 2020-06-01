fn main() {
    let mut args = std::env::args();

    let lookup_glob = args.nth(1).expect("you must pass a glob");
    let verbose = match args.next().as_deref() {
        Some("-v") | Some("--verbose") => true,
        _ => false,
    };

    if verbose {
        println!("searching in {}...", lookup_glob);
    }

    for file in glob::glob(&lookup_glob).expect("got the path right!") {
        let path = file.expect("you gave me back legit files");

        if verbose {
            println!("processing {}...", &path.display());
        }

        let contents = std::fs::read_to_string(&path).expect("these files are legitimate strings");

        std::fs::rename(&path, &path.with_extension("md.back")).expect(&format!(
            "could not rename {} to {}",
            &path.display(),
            &path.with_extension("md.back").display()
        ));

        std::fs::write(&path, rename_tags(&contents))
            .expect(&format!("could not write {}", path.display()));
    }
}

#[derive(PartialEq)]
enum State {
    Uninteresting,
    PossibleTag,
    Tag,
    PossibleBreak,
}

fn rename_tags(content: &str) -> String {
    // Tags always start with `#z/`. After that, they may:
    //
    // - have *no* spaces, but with any number of `/` within them, and be
    //   *terminated* by spaces
    // - have internal spaces, and be terminated by another `#`

    let mut buffer = Vec::with_capacity(content.len());
    let mut state = State::Uninteresting;
    let mut tag_buffer = Vec::new();

    for c in content.chars() {
        match (&state, c) {
            (&State::Uninteresting, '#') => {
                state = State::PossibleTag;
            }
            (&State::Uninteresting, _) => {
                buffer.push(c);
            }
            (&State::PossibleTag, '#') => {
                buffer.push(c);
                state = State::Uninteresting;
            }
            (&State::PossibleTag, ' ') => {
                buffer.push(c);
                state = State::Uninteresting
            }
            (&State::PossibleTag, _) => {
                tag_buffer.clear(); // *shouldn't* ever be necessary...
                tag_buffer.push('#');
                tag_buffer.push(c);
                state = State::Tag
            }
            (&State::Tag, '\n') => {
                flush_tag_buffer(&mut tag_buffer, &mut buffer);
                buffer.push(c);
                state = State::Uninteresting
            }
            (&State::Tag, ' ') => {
                tag_buffer.push(c);
                state = State::PossibleBreak;
            }
            (&State::Tag, '#') => {
                flush_tag_buffer(&mut tag_buffer, &mut buffer);
                state = State::Uninteresting;
            }
            (&State::Tag, _) => {
                tag_buffer.push(c);
            }
            (&State::PossibleBreak, '\n') => {
                tag_buffer.pop();
                flush_tag_buffer(&mut tag_buffer, &mut buffer);
                buffer.push(c);
                state = State::Uninteresting;
            }
            (&State::PossibleBreak, '#') => {
                tag_buffer.pop();
                flush_tag_buffer(&mut tag_buffer, &mut buffer);
                buffer.push(' ');
                state = State::PossibleTag;
            }
            (&State::PossibleBreak, _) => {
                tag_buffer.push(c);
                state = State::Tag;
            }
        }
    }

    // Handle EOF
    if state == State::Tag {
        flush_tag_buffer(&mut tag_buffer, &mut buffer);
    }

    buffer.iter().collect()
}

fn flush_tag_buffer(tag_buffer: &mut Vec<char>, buffer: &mut Vec<char>) {
    let mut initial = true;
    for t in tag_buffer.drain(..) {
        match (initial, t) {
            (false, _) => buffer.push(replaced(t)),
            (true, 'z') | (true, 'Z') => {}
            (true, '/') => initial = false,
            (true, '#') => buffer.push(replaced(t)),
            (true, _) => unreachable!("Should never have initial + NOT [Zz#_], but: {}", t),
        }
    }
}

fn replaced(c: char) -> char {
    if c == '/' {
        '_'
    } else if c == ' ' {
        '-'
    } else {
        c
    }
}

#[cfg(test)]
mod no_spaces {
    #[cfg(test)]
    mod one_tag {
        use crate::*;

        #[test]
        fn non_nested() {
            let test = r##"this is a note

                #z/potato"##;
            let expected = r##"this is a note

                #potato"##;

            assert_eq!(&rename_tags(test), expected);
        }

        #[test]
        fn nested() {
            let test = r##"this is a note

                #z/breakfast/waffles"##;
            let expected = r##"this is a note

                #breakfast_waffles"##;

            assert_eq!(&rename_tags(test), expected);
        }
    }

    #[cfg(test)]
    mod two_tags {
        use super::super::*;

        #[test]
        fn non_nested() {
            let test = r##"this is a note #z/potato #z/steak"##;
            let expected = r##"this is a note #potato #steak"##;

            assert_eq!(&rename_tags(test), expected);
        }

        #[test]
        fn nested() {
            let test = r##"this is a note #z/breakfast/waffles #z/breakfast/pancakes"##;
            let expected = r##"this is a note #breakfast_waffles #breakfast_pancakes"##;

            assert_eq!(&rename_tags(test), expected);
        }
    }
}

#[cfg(test)]
mod internal_spaces {
    #[cfg(test)]
    mod one_tag {
        use crate::*;

        #[test]
        fn non_nested() {
            let test = r##"this is a note #z/breakfast food#"##;
            let expected = r##"this is a note #breakfast-food"##;

            assert_eq!(&rename_tags(test), expected);
        }

        #[test]
        fn nested() {
            let test = r##"this is a note #z/breakfast food/pancakes#"##;
            let expected = r##"this is a note #breakfast-food_pancakes"##;

            assert_eq!(&rename_tags(test), expected);
        }
    }

    #[cfg(test)]
    mod two_tags {
        use crate::*;

        #[test]
        fn non_nested() {
            let test = r##"this is a note #z/breakfast food#"##;
            let expected = r##"this is a note #breakfast-food"##;

            assert_eq!(&rename_tags(test), expected);
        }

        #[test]
        fn nested() {
            let test = r##"this is a note #z/breakfast food/pancakes# #z/breakfast food/waffles#"##;
            let expected = r##"this is a note #breakfast-food_pancakes #breakfast-food_waffles"##;

            assert_eq!(&rename_tags(test), expected);
        }
    }
}

#[cfg(test)]
mod mixed {
    use crate::*;

    #[test]
    fn non_nested() {
        let test = r##"this is a note #z/breakfast food# #z/potatoes"##;
        let expected = r##"this is a note #breakfast-food #potatoes"##;

        assert_eq!(&rename_tags(test), expected);
    }

    #[test]
    fn nested() {
        let test = r##"this is a note #z/breakfast food/pancakes# #z/potatoes"##;
        let expected = r##"this is a note #breakfast-food_pancakes #potatoes"##;

        assert_eq!(&rename_tags(test), expected);
    }
}
