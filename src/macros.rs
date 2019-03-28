// make moving clones into closures more convenient
#[macro_export]
macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

#[macro_export]
macro_rules! app_id {
    () => {
        crate::globals::APP_ID.unwrap_or("com.github.Cogitri.gxi.devel")
    };
    (id => $e:expr) => {
        crate::globals::APP_ID.unwrap_or(id)
    };
}
