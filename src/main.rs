use clap::{Arg, ArgAction, command, value_parser};
use tokio;

mod crawler;
mod pre_process;

#[tokio::main]
async fn main() {
    let matches = command!().version("1.0.0").author("Ricky1911").about(
        "Download e-book from http://ereserves.lib.tsinghua.edu.cn. By default, the number of processes is four and the temporary images WILL BE preserved. 
        For example, \"thubookrs https://ereserves.lib.tsinghua.edu.cn/bookDetail/c01e1db11c4041a39db463e810bac8f94af518935a1ec46ef --token eyJhb...\". 
        Note that you need to manually login the ereserves website and obtain the token from the FIRST request after login, 
        like \"/index?token=xxx\", due to two-factor authentication (2FA)."
    )
    .arg(Arg::new("url").required(true).value_parser(value_parser!(String)))
    .arg(Arg::new("token").required(true).short('t').long("token").help("Required. The token from the \"/index?token=xxx\".").value_parser(value_parser!(String)))
    .arg(Arg::new("thread_number").required(false).short('n').help("Optional. The number of threads. [1~16]").value_parser(value_parser!(i32).range(1..17)).default_value("4"))
    .arg(Arg::new("quality").required(false).short('q').help("Optional. The quality of the generated PDF. The bigger the value, the higher the resolution. [3~10]").value_parser(value_parser!(i32).range(3..11)).default_value("10"))
    .arg(Arg::new("del_img").required(false).short('d').long("del-img").help("Optional. Delete the temporary images.").action(ArgAction::SetTrue))
    .arg(Arg::new("auto_resize").required(false).short('r').long("auto-resize").help("Optional. Automatically unify page sizes.").action(ArgAction::SetTrue))
    .get_matches();
    let url = matches.get_one::<String>("url").unwrap();
    let token = matches.get_one::<String>("token").unwrap();
    let thread_number = matches.get_one::<i32>("thread_number").unwrap();
    let quality = matches.get_one::<i32>("quality").unwrap();
    let del_img = matches.get_one::<bool>("del_img").unwrap();
    let auto_resize = matches.get_one::<bool>("auto_resize").unwrap();
    let _ = pre_process::get_scan_id(url, token).await.unwrap();
}
