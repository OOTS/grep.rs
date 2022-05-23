use std::fs::File;
use std::io::BufReader;
use std::io::BufRead;

use clap::Parser;

mod hit_handling;
use hit_handling::HitHandler;
use hit_handling::HitPrinter;
use hit_handling::HitCounter;

const PATTERN_HELP : &str = "a glob-style pattern to search for in the given files";
const FILES_HELP : &str = concat!(
    "zero or more paths of files. Each file will be searched for occurences ",
    "of the given pattern. Use \"-\" to search standard input as if it were a file. ",
    "If no files are given, search standard input."
);
const INVERT_MATCH_HELP : &str = concat!(
    "if given, print lines *not* matching the pattern instead of lines ",
    "matching the pattern"
);
const PRINT_FILENAME : &str = "--with-filename";
const PRINT_FILENAME_HELP : &str = concat!(
    "Print the filename of each match. (This is the default behaviour when there is ",
    "more than one file.)"
);
const NO_PRINT_FILENAME : &str = "--no-filename";
const NO_FILENAME_HELP : &str = concat!(
    "Do not print the name of the file of each match, even if there are multiple input files."
);
const LINE_NUMBER_LONG : &str = "--line-number";
const LINE_NUMBER_HELP : &str = concat!(
    "For each hit, print the line number (in the respective file) on which the match occured. ",
    "The line number count is one-based."
);
const FILES_WITHOUT_MATCH_LONG : &str = "--files-without-match";
const FILES_WITHOUT_MATCH_HELP : &str = concat!(
    "Suppress normal output, instead only print the names/paths of the files which do not contain ",
    "a match."
);
const FILES_WITH_MATCH_LONG : &str = "--files-with-match";
const FILES_WITH_MATCH_HELP : &str = concat!(
    "Suppress normal output, instead only print the names/paths of the files which contain a ",
    "match. Processing of each file stops after the first match."
);
const COUNT_LONG : &str = "--count";
const COUNT_HELP : &str = concat!(
    "Suppress normal output, instead print the number of matches for each input file."
);
const ABOUT_TEXT : &str = "search for strings (or patterns) in files";

#[derive(Parser)]
#[clap(about=ABOUT_TEXT)]
struct Args {
    #[clap(help=PATTERN_HELP)]
    pattern : String,
    #[clap(help=FILES_HELP)]
    files: Vec<String>,
    #[clap(short='v', long, help=INVERT_MATCH_HELP)]
    invert_match: bool,
    #[clap(short='H', long=PRINT_FILENAME, help=PRINT_FILENAME_HELP)]
    force_print_filename: bool,
    #[clap(short='h', long=NO_PRINT_FILENAME, help=NO_FILENAME_HELP)]
    force_no_print_filename: bool,
    #[clap(short='n', long=LINE_NUMBER_LONG, help=LINE_NUMBER_HELP)]
    print_line_number : bool,
    #[clap(short='l', long=FILES_WITH_MATCH_LONG, help=FILES_WITH_MATCH_HELP)]
    print_matching_files: bool,
    #[clap(short='L', long=FILES_WITHOUT_MATCH_LONG, help=FILES_WITHOUT_MATCH_HELP)]
    print_non_matching_files: bool,
    #[clap(short='c', long=COUNT_LONG, help=COUNT_HELP)]
    count_hits_per_file: bool,
}

fn main() {

    // Argument parsing and sanity checking, setup

    let mut args = Args::parse();

    if args.files.is_empty() {
        args.files.push(String::from("-"));
    }

    let pattern = glob::ParsedGlobString::try_from(&args.pattern[..]);
    if pattern.is_err() {
        //FIXME: better error handling
        println!("could not parse pattern: {:?}", pattern.unwrap_err());
        return
    }
    let pattern = pattern.unwrap();

    let mut normal_output = true;
    let mut print_files = args.files.len() > 1;
    let print_lines = args.print_line_number;
    let print_hit = true;
    let mut skip_file_after_first_match = false;

    match (args.force_print_filename, args.force_no_print_filename) {
        (true, true) => {
            //FIXME: better error handling
            println!("conflicting command line options: {} and {} (or equivalents)", PRINT_FILENAME, NO_PRINT_FILENAME);
            return
        }
        (false, true) => { print_files = false; }
        (true, false) => { print_files = true; }
        (false, false) => { }
    }

    match (args.print_matching_files, args.print_non_matching_files, args.count_hits_per_file) {
        (true, true, _) | (true, _, true) | (_, true, true) => {
            //FIXME: better error handling
            println!("conflicting command line options: only one of {}, {} and {} (or equivalents) may be given",
                     FILES_WITH_MATCH_LONG, FILES_WITHOUT_MATCH_LONG, COUNT_LONG);
            return;
        }
        (true, false, false) => {
            normal_output = false;
            skip_file_after_first_match = true;
        }
        (false, true, false) => {
            normal_output = false;
        }
        (false, false, true) => {
            normal_output = false;
        }
        (false, false, false) => { /* NOP */ }
    }

    let mut hit_counter = match args.count_hits_per_file || args.print_non_matching_files || args.print_matching_files {
        true => Some(HitCounter::new()),
        false => None
    };

    let mut hit_printer : Option<HitPrinter> = match normal_output {
        true => Some(HitPrinter::new(print_files, print_lines, print_hit)),
        false => None,
    };

    {
        let mut hit_handlers : Vec<Box<&mut dyn HitHandler>> = Vec::new();
        hit_printer.as_mut().and_then(|mut_ref| Some(hit_handlers.push(Box::new(mut_ref))));
        hit_counter.as_mut().and_then(|mut_ref| Some(hit_handlers.push(Box::new(mut_ref))));

        // search the input files

        let stdin = std::io::stdin();
        for file_path in &args.files {
            let reader: Box<dyn BufRead>;
            match &file_path[..] {
                "-" => {
                    reader = Box::new(stdin.lock());
                    //reader.lock();
                }
                _ => {
                    let file = File::options().read(true).open(&file_path);
                    if file.is_err() {
                        //FIXME: better error handling
                        println!("could not open file {}", file_path);
                        return
                    }
                    let file = file.unwrap();
                    reader = Box::new(BufReader::new(file));
                }
            }

            for hit_handler in hit_handlers.iter_mut() {
                hit_handler.start_new_file(file_path);
            }

            for (line_no, line) in reader.lines().enumerate() {
                let line = line.unwrap();
                let mut matches = pattern.matches_partially(&line);
                if args.invert_match {
                    matches = !matches;
                }
                if matches {
                    for hit_handler in hit_handlers.iter_mut() {
                        hit_handler.handle_hit(file_path, line_no + 1, &line);
                    }
                    if skip_file_after_first_match {
                        break
                    }
                }
            }

        } // end of lifetime of hit_handlers, we now have full ownership of the hit handlers again

        if args.print_matching_files {
            let hit_counter = hit_counter.as_ref().unwrap();
            for (file, _count) in hit_counter.iter().filter(|(_file, count)| *count > 0) {
                println!("{}", file);
            }
        };

        if args.print_non_matching_files {
            let hit_counter = hit_counter.as_ref().unwrap();
            for (file, _count) in hit_counter.iter().filter(|(_file, count)| *count == 0) {
               println!("{}", file);
            }
        };

        if args.count_hits_per_file {
            let hit_counter = hit_counter.as_ref().unwrap();
            for (file, count) in hit_counter {
                println!("{}:{}", file, count);
            }
        };

    }

}
