# gxi-peer

gxi's way to spawn a Xi instance in a thread and connect to it. It saves messages it receives from Xi in a SharedQueue
(a fancy crossbeam_queue::SegQueue with a bit of comfort functionality added), to later process each one of them in a
synchronous manner in gxi's MainWin.

## Contributing

Please see the docs on https://gxi.cogitri.dev/docs to learn more about gxi's inner workings. 
[gtk-rs' site](https://gtk-rs.org/) offers documentation and examples about how gtk-rs works.

Visit [Weblate](https://hosted.weblate.org/engage/gxi/) to translate gxi and its components.