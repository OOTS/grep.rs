use std::collections::BTreeMap;
use std::collections::btree_map;

pub trait HitHandler<'f> {
    fn start_new_file(&mut self, file_path: &'f str);
    fn handle_hit(&mut self, file_path: &'f str, line: usize, hit: &str);
}

pub struct HitPrinter {
    print_file_path: bool,
    print_line: bool,
    print_hit: bool,
}

impl HitPrinter {
    pub fn new(print_file_path: bool, print_line: bool, print_hit: bool) -> Self {
        HitPrinter {
            print_file_path: print_file_path,
            print_line: print_line,
            print_hit: print_hit
        }
    }
}

impl<'f> HitHandler<'f> for HitPrinter {
    #[allow(unused_variables)]
    fn start_new_file(&mut self, file_path: &'f str) { /* NOP */ }
    #[allow(unused_assignments)]
    fn handle_hit(&mut self, file_path: &'f str, line: usize, hit: &str) {
        let mut have_content = false;
        if self.print_file_path {
            print!("{}", file_path);
            have_content = true;
        }
        if self.print_line {
            if have_content { print!(":") };
            print!("{}", line);
            have_content = true;
        }
        if self.print_hit {
            if have_content { print!(":"); };
            print!("{}", hit);
            have_content = true;
        }
        print!("\n");
    }
}

pub struct HitCounter<'f> {
    hits: BTreeMap<&'f str, usize>
}

impl<'a, 'f> HitCounter<'f> {
    pub fn new() -> Self { HitCounter { hits : BTreeMap::new() } }
    pub fn iter(&'a self) -> HitCounterIter<'a, 'f> {
        self.into_iter()
    }
}
impl<'f> HitHandler<'f> for HitCounter<'f> {
    fn start_new_file(&mut self, file_path: &'f str) {
        self.hits.insert(file_path, 0);
    }
    #[allow(unused_variables)]
    fn handle_hit(&mut self, file_path: &'f str, line: usize, hit: &str) {
        *self.hits.get_mut(file_path).unwrap() += 1;
    }
}
impl<'a, 'f> IntoIterator for &'a HitCounter<'f> {
    type Item = (&'f str, usize);
    type IntoIter = HitCounterIter<'a, 'f>;
    fn into_iter(self) -> Self::IntoIter {
        HitCounterIter::from_hit_counter(self)
    }
}

// utility struct to allow iterating over items of type
// (&'f str, usize) instead of (&'a &'f str, &'a usize)
pub struct HitCounterIter<'a, 'f> {
    iter: btree_map::Iter<'a, &'f str, usize>
}
impl<'a, 'f> HitCounterIter<'a, 'f> {
    fn from_hit_counter(hit_counter: &'a HitCounter<'f>) -> Self {
        HitCounterIter { iter: (&hit_counter.hits).into_iter() }
    }
}
impl<'a, 'f> Iterator for HitCounterIter<'a, 'f> {
    type Item = (&'f str, usize);
    fn next(&mut self) -> Option<Self::Item>{
        self.iter.next().map(|(k, v)| (*k, *v))
    }
}