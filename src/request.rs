#[derive(Clone, Debug)]
pub enum Request {
    NewView{file_path: Option<String>},
}
