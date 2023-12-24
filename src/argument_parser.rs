type ArgumentMapFunction<T> = fn(&[String], &mut T) -> Result<(), String>;
#[derive(derive_new::new, Clone)]
pub struct ArgumentMapping<T> {
    pub letter: char,
    pub map_fnc: ArgumentMapFunction<T>,
}

pub fn parse_arguments<T>(
    args: &[String],
    ctor: impl Fn() -> T,
    main_map: impl Fn(Option<&str>, &mut T) -> Result<(), String>,
    mappers: Vec<ArgumentMapping<T>>,
) -> Result<T, String> {
    if args.len() == 0 {
        return Ok(ctor());
    }
    let mut config = ctor();
    let has_main_arg = !args[0].starts_with("-");
    main_map(
        {
            if has_main_arg {
                Some(&args[0])
            } else {
                None
            }
        },
        &mut config,
    )?;
    let start_index = if has_main_arg { 1 } else { 0 };

    let mut collect_start_index = 0;
    let mut maybe_mapper: Option<&ArgumentMapping<T>> = None;
    let mut already_done: Vec<bool> = Vec::new();
    already_done.resize(mappers.len(), false);

    for i in start_index..args.len() {
        if maybe_mapper.is_none() {
            let chars: Vec<char> = args[i].chars().collect();
            if chars.len() != 2 || chars[0] != '-' {
                return Err(
                    "Argument passing doesn't follow format \"-{{letter}}content\" ".to_owned(),
                );
            }
            for (el_index, el) in mappers.iter().enumerate() {
                if el.letter == chars[1] {
                    if already_done[el_index] {
                        return Err(format!("Specified twice: {}", args[i]));
                    }
                    already_done[el_index] = true;
                    collect_start_index = i + 1;
                    maybe_mapper = Some(el);
                    break;
                }
            }
            if maybe_mapper.is_none() {
                return Err(format!("Unknown argument kind : {}", args[i]));
            }
        }

        let mapper = maybe_mapper.unwrap();
        if i + 1 == args.len() || args[i + 1].starts_with("-") {
            (mapper.map_fnc)(&args[collect_start_index..=i], &mut config)?;
            maybe_mapper = None;
        }
    }

    return Ok(config);
}
