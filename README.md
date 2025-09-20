# thubookrs

使用rust编写的命令行工具。

用于下载清华教参平台上的电子书pdf版本，清华教参平台：https://ereserves.lib.tsinghua.edu.cn/ 。

## 背景

本项目是用rust对 [TsinghuaBookCrawler](https://github.com/dylanyang17/TsinghuaBookCrawler) 项目进行的重写，在保留其原有的全部功能的同时部分提升了性能，并且可以开箱即用无需配置环境。

代码上很多参考了原项目，感谢原项目开发者 [dylanyang17](https://github.com/dylanyang17) 允许我借鉴其代码，同时一并感谢原项目的其他贡献者。

此工具**仅供清华师生学习**之用，请在使用过程中注意版权问题。使用此工具造成的一切不良影响与作者无关。

## 下载

### 使用二进制文件（推荐）

在[GitHub Releases](https://github.com/Ricky1911/thubookrs/releases)中找到最新的版本下载并解压。

### 使用源代码编译（不推荐）

本项目只提供了用于Windows的可执行文件，如果想要在Linux等系统上使用或对本项目进行二次开发，请自行下载源代码并编译。

你应当提前安装[git](https://git-scm.com/downloads)以及[rust工具链](https://www.rust-lang.org/)。

使用以下命令下载代码：
```
git clone https://github.com/Ricky1911/thubookrs
```

进入Cargo.toml所在目录，运行以下命令编译：
```
cargo build -r
```

## 使用

### 基础使用

在文件thubookrs.exe所在的目录打开命令行，输入```thubookrs --help```可获取帮助信息。
```
Usage: thubookrs.exe [OPTIONS] --token <token> <url>

Arguments:
  <url>

Options:
  -t, --token <token>     Required. The token from the "/index?token=xxx".
  -n <thread_number>      Optional. The number of threads. [1~16] [default: 4]
  -q <quality>            Optional. The quality of the generated PDF. The bigger the value, the higher the resolution. [3~10] [default: 10]
  -d, --del-img           Optional. Delete the temporary images.
  -r, --auto-resize       Optional. Automatically unify page sizes.
  -h, --help              Print help
  -V, --version           Print version
```

一般需求只需使用-t参数传入token。

假设要下载[这本书](https://ereserves.lib.tsinghua.edu.cn/bookDetail/c01e1db11c4041a39db463e810bac8f9)，只需输入以下命令：

```
./thubookrs https://ereserves.lib.tsinghua.edu.cn/bookDetail/c01e1db11c4041a39db463e810bac8f9
4af518935a1ec46ef -t eyJhb...
```

其中第一个参数是书籍的详情页面的链接，一般是https://ereserves.lib.tsinghua.edu.cn/bookDetail/xxx 。

第二个参数是用户登录使用的token，可以通过以下方式获取：

打开浏览器并访问教参平台，在统一身份认证页面按F12打开开发者工具，选择网络，随后在网页中正常登录。

登录完成后在网络中可看到一条index?xxx的记录，将xxx的值复制即可。

运行完毕后会在运行目录下的downloads文件夹中输出最终pdf文件。

### 高级使用

使用-n参数控制用于下载图片的线程数。

使用-q参数调整最终pdf中图片的清晰度。

使用-d参数在转换完成pdf之后自动删除下载的图片。

使用-r参数自动统一图片尺寸。

## 说明

欢迎各位开发者为本项目添砖加瓦，也欢迎各位同学使用本工具并提出修改意见。

能力有限，仓促写就，如有代码问题，欢迎批评指正。

作者邮箱：ricky_1911@163.com