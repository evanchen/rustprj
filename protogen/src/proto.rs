use conf::conf;
use core::panic;
use rand::Rng;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::Result;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{channel, Sender};
use std::{fs, io, thread};
extern crate md5;
extern crate rand;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IType {
    Primitive,
    Datatype,
    Protocol,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WireType {
    Vint = 0,
    Double = 1,
    Repeated = 2,
    _Deprecated1 = 3,
    _Deprecated2 = 4,
    Float = 5,
}

impl WireType {
    fn type_to_number(&self) -> u8 {
        match self {
            WireType::Vint => 0,
            WireType::Double => 1,
            WireType::Repeated => 2,
            WireType::_Deprecated1 => 3,
            WireType::_Deprecated2 => 4,
            WireType::Float => 5,
        }
    }
}

#[derive(Debug)]
pub struct LineInfo {
    name: String,                    // message 结构体名字
    wirename: String,                // 字段对应在 rust 的类型,或 datataype 的 message 结构体名字
    literal: String,                 // 字符串字面量
    wiretype: WireType,              // wire type
    id: i32,                         // 协议id
    repeated: bool,                  // 是否是数组
    embed: Option<Rc<RefCell<Pto>>>, // 在具体解析时,bool 和 vint 共享 Vint; string 和 datatype 共享 Repeated
}

impl LineInfo {
    pub fn new(
        name: String,
        wirename: String,
        literal: String,
        wiretype: WireType,
        id: i32,
        repeated: bool,
    ) -> LineInfo {
        LineInfo {
            name,
            wirename,
            literal,
            wiretype,
            id,
            repeated,
            embed: None,
        }
    }
}

#[derive(Debug)]
pub struct Pto {
    name: String,
    itype: IType,
    members: Vec<LineInfo>,
}

impl Pto {
    pub fn new(name: String, itype: IType) -> Pto {
        Pto {
            name,
            itype,
            members: Vec::new(),
        }
    }
}

type Dtmap = HashMap<String, Rc<RefCell<Pto>>>;

pub fn parse_proto() {
    let sysconf = conf::Conf::new();
    let ptosrc = sysconf.get_src_dir();
    let ptoout = sysconf.get_out_dir();
    let initprotos = sysconf.get_init_protos();
    let (tx, rx) = channel::<(IType, PathBuf)>();
    thread::spawn(move || {
        let primitivedir = format!("{}/primitive", ptosrc);
        let datatypedir = format!("{}/datatype", ptosrc);
        let protocoldir = format!("{}/protocol", ptosrc);
        walk_dir(&primitivedir, &tx, IType::Primitive).expect("walk_dir primitivedir failed");
        walk_dir(&datatypedir, &tx, IType::Datatype).expect("walk_dir datatypedir failed");
        walk_dir(&protocoldir, &tx, IType::Protocol).expect("walk_dir protocoldir failed");
    });
    let mut map_primitive = Dtmap::new();
    let mut map_datatype = Dtmap::new();
    let mut map_pto = Dtmap::new();
    while let Ok((itype, fname)) = rx.recv() {
        match itype {
            IType::Primitive => {
                srcfile2structs(itype, &fname, &mut map_primitive);
            }
            IType::Datatype => {
                srcfile2structs(itype, &fname, &mut map_datatype);
            }
            IType::Protocol => {
                srcfile2structs(itype, &fname, &mut map_pto);
            }
        }
    }
    analyze_structs(&map_primitive, &mut map_datatype, &mut map_pto);
    generate(initprotos, &ptoout, &mut map_datatype, &mut map_pto);
    println!("parse_proto is ready.");
}

fn walk_dir(
    srcdir: &dyn AsRef<Path>,
    tx: &Sender<(IType, PathBuf)>,
    itype: IType,
) -> io::Result<()> {
    for entry in fs::read_dir(srcdir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, tx, itype)?;
        } else if path.is_file() {
            tx.send((itype, path.clone())).unwrap();
        }
    }
    Ok(())
}

//str should be a "=" line
fn line_strip(str: &str) -> LineInfo {
    let (str1, str2) = str.split_once("=").unwrap();
    let str2 = str2.trim();
    assert!(!str2.is_empty());
    let str1 = str1.trim();
    assert!(!str1.is_empty());
    let id: i32 = str2.parse().unwrap();
    assert!(id > 0);
    let mut vs: Vec<&str> = str1.split_whitespace().collect();
    if !(vs.len() == 2 || (vs.len() == 3 && vs[0] == "repeated")) {
        panic!("one line block error: {}", str);
    }
    let repeated = if vs.len() == 2 {
        false
    } else {
        vs.remove(0);
        true
    };
    //把字符串字面量转换成 wiretype, 在解析时找对应的 wiretype 即可.
    let literal = vs[0];
    let (wiretype, wirename) = match literal {
        "int" => (WireType::Vint, "i32"),
        "int8" => (WireType::Vint, "i8"),
        "uint8" => (WireType::Vint, "u8"),
        "int16" => (WireType::Vint, "i16"),
        "uint16" => (WireType::Vint, "u16"),
        "int32" => (WireType::Vint, "i32"),
        "uint32" => (WireType::Vint, "u32"),
        "int64" => (WireType::Vint, "i64"),
        "uint64" => (WireType::Vint, "u64"),
        "bool" => (WireType::Vint, "bool"),
        "float" => (WireType::Float, "f32"),
        "double" => (WireType::Double, "f64"),
        "string" => (WireType::Repeated, "String"),
        _ => (WireType::Repeated, literal),
    };
    let name = vs[1];
    LineInfo::new(
        name.to_owned(),
        wirename.to_owned(),
        literal.to_owned(),
        wiretype,
        id,
        repeated,
    )
}

// :TODO: 仅支持行注释 "//", 不支持段块注释 "/*...*/"
fn srcfile2structs(itype: IType, path: &Path, dm: &mut Dtmap) {
    //println!("[srcfile2structs]: {}, {}",itype,path.display());
    let fname = path.file_name().unwrap();
    let fname = Path::new(fname).file_stem().unwrap().to_str().unwrap();
    println!("{}", fname);

    let file = File::open(path).unwrap();
    let mut unique_id = HashMap::<i32, bool>::new();
    let mut unique_name = HashMap::<String, bool>::new();
    let mut vs = Vec::new(); // 过滤掉所有注释后的文本行
    for line in io::BufReader::new(file).lines() {
        match line {
            Ok(str0) => {
                let str = if str0.contains("//") {
                    let (str1, _) = str0.split_once("//").unwrap();
                    str1.to_owned()
                } else {
                    str0
                };
                if str.trim().is_empty() {
                    continue;
                }
                vs.push(str);
            }
            Err(err) => {
                println!("{}", err);
            }
        }
    }
    //由于可能出现一行的 block, 把所有行用分隔符" "合并为一行字符串,再分解
    let str = vs.join(" ");
    let (head, body) = str.split_once("{").unwrap();
    assert!(body.ends_with('}'));
    let body = body.strip_suffix('}').unwrap();

    // get the message name
    let vs: Vec<&str> = head.trim().split_whitespace().collect();
    assert_eq!(vs.len(), 2);
    assert_eq!(vs[0], "message");
    let name = vs[1];
    assert!(name == fname); //文件名与 message 名字必须相同.

    let pto = Rc::new(RefCell::new(Pto::new(name.to_owned(), itype)));
    dm.insert(name.to_owned(), pto.clone());

    // if this line has more fields, every line should be ended with ";"
    let vs: Vec<&str> = body.split(';').collect();
    for str in vs {
        let str = str.trim();
        if str.is_empty() {
            continue;
        }
        let lineinfo = line_strip(str);
        let id = lineinfo.id;
        let name = lineinfo.name.clone();
        assert_eq!(unique_id.insert(id, true), None);
        assert_eq!(unique_name.insert(name, true), None);
        pto.borrow_mut().members.push(lineinfo);
    }
}

fn recursive_clone(
    parentname: &str,
    map_primitive: &Dtmap,
    lineinfo: &mut LineInfo,
    mcopy: &Dtmap,
    depth: i32,
) {
    assert!(depth <= 10);
    if lineinfo.embed.is_some() {
        return;
    }
    //println!("{}-{}-{}",depth,lname,lineinfo.literal);
    if lineinfo.wiretype == WireType::Repeated && lineinfo.literal != "string" {
        if let Some(res) = mcopy.get(&lineinfo.literal) {
            //内嵌 datatype
            for lineinfo in res.borrow_mut().members.iter_mut() {
                // 如果存在环形引用, RefCell 的动态借用规则会报错.
                recursive_clone(parentname, map_primitive, lineinfo, mcopy, depth + 1);
            }
            lineinfo.embed = Some(res.clone());
        } else {
            panic!(
                "no such literal: {}-{}-{}",
                parentname, lineinfo.name, lineinfo.literal
            );
        }
    } else if let Some(res) = map_primitive.get(&lineinfo.literal) {
        //找 primitive, literal 就是关键字而不是字段名
        lineinfo.embed = Some(res.clone());
    } else {
        panic!(
            "no such literal: {}-{}-{}",
            parentname, lineinfo.name, lineinfo.literal
        );
    }
}

fn analyze_structs(map_primitive: &Dtmap, map_datatype: &mut Dtmap, map_pto: &mut Dtmap) {
    //println!("map_primitive: {:?}\n\n", map_primitive);
    //println!("map_datatype: {:?}\n\n", map_datatype);
    //println!("map_pto: {:?}\n\n", map_pto);

    // map_datatype 可以有多层嵌套,但我们得有个层数限制,并且防范环形引用
    //println!("\nmap_datatype:");
    for (_k, v) in map_datatype.iter() {
        let parentname = v.borrow_mut().name.clone();
        for lineinfo in v.borrow_mut().members.iter_mut() {
            recursive_clone(&parentname, map_primitive, lineinfo, map_datatype, 1);
        }
        //println!("\n\n");
    }
    // map_pto 只有一层,每一行不是 primitive 就是 datatype
    //println!("\nmap_pto:");
    for (_k, v) in map_pto.iter() {
        let parentname = v.borrow_mut().name.clone();
        for lineinfo in v.borrow_mut().members.iter_mut() {
            assert!(lineinfo.embed.is_none());
            recursive_clone(&parentname, map_primitive, lineinfo, map_datatype, 1);
        }
        //println!("\n\n");
    }
    //println!("map_pto: {:?}\n\n", map_pto);
}

fn generate(initprotos: &[String], outdir: &str, map_datatype: &mut Dtmap, map_pto: &mut Dtmap) {
    let mut struct_names = Vec::<String>::new();
    //生成 datatype struct
    for (_k, v) in map_datatype.iter() {
        datatype2file(outdir, v).unwrap();
        let name = v.borrow_mut().name.clone();
        struct_names.push(name);
    }

    //生成 protocol struct
    let mut allptos = Vec::new();
    let mut ptoid = 100u32; // 前 100 是保留用
    for name in initprotos {
        if let Some((_, pto)) = map_pto.remove_entry(name) {
            ptoid += 1;
            let name = pto.borrow_mut().name.clone();
            struct_names.push(name.clone());
            allptos.push((ptoid, name));
            pto2file(outdir, ptoid, &pto).unwrap();
        } else {
            panic!("no such init_protos: {}", name);
        }
    }
    let mut others: Vec<&Rc<RefCell<Pto>>> = map_pto.values().collect();
    let mut inorder = Vec::new();
    let size = others.len();
    let mut rng = rand::thread_rng();
    for _i in 0..size {
        let idx = rng.gen_range(0..others.len());
        inorder.push(others.remove(idx));
    }
    assert!(inorder.len() == size);
    ptoid = 200; // ptoid为 101 到 200 的顺序是固定不变的, 201 之后的顺序是随机的
    for pto in inorder {
        ptoid += 1;
        let name = pto.borrow_mut().name.clone();
        struct_names.push(name.clone());
        allptos.push((ptoid, name));
        pto2file(outdir, ptoid, pto).unwrap();
    }
    assert!(ptoid <= 65535); // u16

    generate_all_pto_mapping(&allptos, outdir, "allptos.rs").unwrap();

    // generate mod.rs
    let fname = format!("{}/mod.rs", outdir);
    let mut file = File::create(fname).unwrap();
    //file header
    write_file_header(&mut file).unwrap();
    for name in struct_names {
        let linestr = format!("pub mod {};\n", name);
        file.write_all(linestr.as_bytes()).unwrap();
    }
    let linestr = format!("pub mod {};\n", "allptos");
    file.write_all(linestr.as_bytes()).unwrap();
}

fn datatype2file_empty(outdir: &str, pto: &Rc<RefCell<Pto>>) -> Result<()> {
    let struct_name = pto.borrow_mut().name.clone();
    let fname = format!("{}/{}.rs", outdir, struct_name);
    let mut file = File::create(fname).unwrap();
    //file header
    write_file_header(&mut file)?;
    //a whole empty structure
    let wholestruct = format!(
        r#"
use crate::{{MsgRead, MsgWrite, BytesReader, BytesWriter, Error, Result}};
use crate::sizeofs;
use crate::util;

#[derive(Debug,Default)]
pub struct {0} {{

}}

impl {0} {{
    pub fn default_with_random_value() -> Self {{
        Self::default()
    }}
}}

impl MsgRead for {0} {{
    fn read(_r: &mut BytesReader, _bytes: &[u8]) -> Result<Self> {{
        let msg = {0} {{}};
        Ok(msg)
    }}
}}

impl MsgWrite for {0} {{
    fn size(&self) -> usize {{
        0
    }}
    fn write(&self, _w: &mut BytesWriter) -> Result<()> {{
        Ok(())
    }}
}}
"#,
        struct_name,
    );
    write_line(&mut file, &wholestruct)?;
    write_line(&mut file, "\n\n")?;
    Ok(())
}

fn datatype2file(outdir: &str, pto: &Rc<RefCell<Pto>>) -> Result<()> {
    if pto.borrow_mut().members.is_empty() {
        return datatype2file_empty(outdir, pto);
    }
    let struct_name = pto.borrow_mut().name.clone();
    let fname = format!("{}/{}.rs", outdir, struct_name);
    let mut file = File::create(fname).unwrap();
    //file header
    write_file_header(&mut file)?;
    //imports
    let mut embednames = HashMap::new();
    for lineinfo in &pto.borrow_mut().members {
        if let Some(embed) = &lineinfo.embed {
            if embed.borrow_mut().itype == IType::Datatype {
                let names = embed.borrow_mut().name.clone();
                let str = format!("use crate::{}::{}", names, names);
                embednames.insert(str, true);
            }
        }
    }
    let line = if embednames.is_empty() {
        r#"
use crate::{MsgRead, MsgWrite, BytesReader, BytesWriter, Error, Result};
use crate::sizeofs;
use crate::util;
"#
        .to_owned()
    } else {
        let keys: Vec<String> = embednames.into_keys().collect();
        let str = keys.join(", ");
        format!(
            r#"
{};
use crate::{{MsgRead, MsgWrite, BytesReader, BytesWriter, Error, Result}};
use crate::sizeofs;
use crate::util;
"#,
            str
        )
    };

    write_line(&mut file, &line)?;
    let mut body = Vec::new();
    let mut rand_body = Vec::new();
    let mut impl_read_body = Vec::new();
    let mut impl_write_body = Vec::new();
    let mut impl_size_body = Vec::new();
    let mut tap = "";
    let mut rtap = "";
    for lineinfo in &pto.borrow_mut().members {
        let mut is_embed_datatype = false;
        let mut needand = "";
        if let Some(embed) = &lineinfo.embed {
            if embed.borrow_mut().itype == IType::Datatype {
                is_embed_datatype = true;
            }
        }
        let tag = (lineinfo.id << 3) | (lineinfo.wiretype.type_to_number() & 0x7) as i32;
        let linename = &lineinfo.name;
        let wirename = &lineinfo.wirename;
        let wirename_func = if wirename == "String" {
            needand = "&";
            "string"
        } else {
            wirename
        };
        let literal = &lineinfo.literal;

        if !lineinfo.repeated {
            let str = format!("    pub {}: {},", linename, wirename);
            body.push(str);

            if !is_embed_datatype {
                // random default
                let str = format!(
                    "\t\tmsg.{} = util::default_random_value(\"{}\").parse().unwrap();",
                    linename, wirename
                );
                rand_body.push(str);

                //read
                let str = format!(
                    "{}Ok({}) => {{ msg.{} = r.read_{}(bytes)?; }}",
                    rtap, tag, linename, wirename_func
                );
                impl_read_body.push(str);

                //write
                let str = format!(
                    "{}w.write_{}_with_tag({},{}self.{})?;",
                    tap, wirename_func, tag, needand, linename
                );
                impl_write_body.push(str);

                //size
                let str = format!(
                    "{}sizeofs::sizeof_tag({}) + sizeofs::sizeof_{}({}self.{})",
                    tap, tag, wirename_func, needand, linename
                );
                impl_size_body.push(str);
            } else {
                // random default
                let str = format!(
                    "\t\tmsg.{} = {}::default_with_random_value();",
                    linename, literal
                );
                rand_body.push(str);

                //read
                let str = format!(
                    "{}Ok({}) => {{ let objsize = r.get_len(bytes)?; let mut nextr = BytesReader::new(r.get_read_start(),r.get_read_start()+objsize); msg.{} = {}::read(&mut nextr,bytes)?; r.step(objsize); }}",
                    rtap, tag, linename, wirename
                );
                impl_read_body.push(str);

                //write
                let str = format!("{}w.write_tag({})?;", tap, tag);
                impl_write_body.push(str);
                let str = format!(
                    "{}let objsize = self.{}.size(); w.write_len(objsize)?; self.{}.write(w)?;",
                    tap, linename, linename
                );
                impl_write_body.push(str);

                //size
                let str = format!(
                    "{}sizeofs::sizeof_tag({}) + {{ let objsize = self.{}.size(); sizeofs::sizeof_len(objsize) + objsize }}",
                    tap, tag, linename
                );
                impl_size_body.push(str);
            }
        } else {
            let str = format!("    pub {}: Vec<{}>,", linename, wirename);
            body.push(str);

            if !is_embed_datatype {
                // random default
                let randlen = rand::thread_rng().gen_range(10..100);
                let str = format!(
                    r#"        let len = {};
        for _idx in 0..len {{
            let val = util::default_random_value("{}").parse().unwrap();
            msg.{}.push(val);
        }}"#,
                    randlen, wirename, linename
                );
                rand_body.push(str);

                //read
                let str = format!(
                    "let val = r.read_{}(bytes)?; msg.{}.push(val);",
                    wirename_func, linename
                );
                let str = format!(
                    "{}Ok({}) => {{ let len = r.get_len(bytes)?; for _idx in 0..len {{ {} }} }}",
                    rtap, tag, str
                );
                impl_read_body.push(str);

                //write
                let str = format!("{}w.write_tag({})?;", tap, tag);
                impl_write_body.push(str);
                let str = format!(
                    "{}let len = self.{}.len(); w.write_len(len)?;",
                    tap, linename
                );
                impl_write_body.push(str);
                let str = format!(
                    "{}for idx in 0..len {{ w.write_{}({}self.{}[idx])?;}}",
                    tap, wirename_func, needand, linename
                );
                impl_write_body.push(str);

                //size
                let str = format!(
                    "{}sizeofs::sizeof_tag({}) + sizeofs::sizeof_len(self.{}.len())",
                    tap, tag, linename
                );
                impl_size_body.push(str);
                let str = format!("{}{{ let mut total = 0; for idx in 0..self.{}.len() {{ total += sizeofs::sizeof_{}({}self.{}[idx]); }} total }}",tap,linename,wirename_func,needand,linename);
                impl_size_body.push(str);
            } else {
                // random default
                let randlen = rand::thread_rng().gen_range(10..100);
                let str = format!(
                    r#"        let len = {};
        for _idx in 0..len {{
            let val = {}::default_with_random_value();
            msg.{}.push(val);
        }}"#,
                    randlen, wirename, linename
                );
                rand_body.push(str);

                //read
                let str = format!(
                    "let objsize = r.get_len(bytes)?; let mut nextr = BytesReader::new(r.get_read_start(),r.get_read_start()+objsize); let val = {}::read(&mut nextr,bytes)?; msg.{}.push(val); r.step(objsize);",
                    literal, linename
                );
                let str = format!(
                    "{}Ok({}) => {{ let len = r.get_len(bytes)?; for _idx in 0..len {{ {} }} }}",
                    rtap, tag, str
                );
                impl_read_body.push(str);

                //write
                let str = format!("{}w.write_tag({})?;", tap, tag);
                impl_write_body.push(str);
                let str = format!(
                    "{}let len = self.{}.len(); w.write_len(len)?;",
                    tap, linename
                );
                impl_write_body.push(str);
                let str = format!(
                    "{}for idx in 0..len {{ let objsize = self.{}[idx].size(); w.write_len(objsize)?; self.{}[idx].write(w)?; }}",
                    tap, linename,linename
                );
                impl_write_body.push(str);

                //size
                let str = format!(
                    "{}sizeofs::sizeof_tag({}) + sizeofs::sizeof_len(self.{}.len())",
                    tap, tag, linename
                );
                impl_size_body.push(str);
                let str = format!("{}{{ let mut total = 0; for idx in 0..self.{}.len() {{ let objsize = self.{}[idx].size(); total += sizeofs::sizeof_len(objsize) + objsize; }} total }}",tap,linename,linename);
                impl_size_body.push(str);
            }
        }
        tap = "        ";
        rtap = "                ";
    }
    let str = format!("{}Ok(t) => {{ println!(\"[read]: {} get an unknow member: {{}}\",t); r.read_unknow(bytes,t)?; }}",rtap,struct_name);
    impl_read_body.push(str);
    let str = format!("{}Err(e) => {{ return Err(e); }}", rtap);
    impl_read_body.push(str);

    let body = body.join("\n");
    //struct body
    write_struct(&mut file, &struct_name, &body)?;
    write_line(&mut file, "\n\n")?;

    // with random default
    let random_default_body = rand_body.join("\n");
    write_imp_struct_with_random_default(&mut file, &struct_name, &random_default_body)?;
    write_line(&mut file, "\n\n")?;

    // trait MsgRead
    let read_body = format!(
        r#"    fn read(r: &mut BytesReader, bytes: &[u8]) -> Result<Self> {{
        let mut msg = Self::default();
        while !r.is_eof() {{
            match r.next_tag(bytes) {{
                {}
            }}
        }}
        Ok(msg)
    }}"#,
        impl_read_body.join("\n"),
    );
    write_imp_read_for_struct(&mut file, &struct_name, &read_body)?;
    write_line(&mut file, "\n\n")?;

    // trait MsgWrite
    let write_body = format!(
        r#"    fn size(&self) -> usize {{
        {}
    }}
    fn write(&self, w: &mut BytesWriter) -> Result<()> {{
        {}
        Ok(())
    }}"#,
        impl_size_body.join(" +\n"),
        impl_write_body.join("\n"),
    );
    write_imp_write_for_struct(&mut file, &struct_name, &write_body)?;
    write_line(&mut file, "\n\n")?;

    generate_test_func(outdir, pto)?;

    Ok(())
}

fn pto2file(outdir: &str, _ptoid: u32, pto: &Rc<RefCell<Pto>>) -> Result<()> {
    datatype2file(outdir, pto)
}

fn generate_all_pto_mapping(allptos: &[(u32, String)], outdir: &str, fname: &str) -> Result<()> {
    let fname = format!("{}/{}", outdir, fname);
    let mut file = File::create(fname).unwrap();

    //file header
    write_file_header(&mut file)?;
    //imports
    write_line(
        &mut file,
        r#"
use std::collections::HashMap;
use std::default::Default;
use crate::{
    ptoout::*,
    BytesReader,
    MsgRead,
    BytesWriter,
    MsgWrite,
};

"#,
    )?;

    let mut vs = Vec::new();
    let mut f2vs = Vec::new();
    let mut f3vs = Vec::new();
    for (id, name) in allptos {
        let str = format!("    {0}({0}::{0}),", name);
        vs.push(str);

        // parse_proto
        f2vs.push(format!("        {} => {{", id));
        f2vs.push("            let mut r = BytesReader::new(start_pos,end_pos);".to_string());
        f2vs.push(format!(
            "            let obj = {0}::{0}::read(&mut r, buf)?;",
            name
        ));
        f2vs.push("            if !r.is_complete() { return Err(crate::Error::Message(format!(\"[allptos.parse_proto]: partial parsed, proto_id={}\",proto_id))) }".to_string());
        f2vs.push(format!("            Ok(ProtoType::{0}(obj))", name));
        f2vs.push("        },".to_string());

        // serialize
        f3vs.push(format!("        ProtoType::{}(obj) => {{", name));
        f3vs.push("            let msglen = obj.size();".to_string());
        f3vs.push("            let mut buf = Vec::with_capacity(msglen);".to_string());
        f3vs.push("            let mut w = BytesWriter::new(&mut buf);".to_string());
        f3vs.push("            obj.write(&mut w)?;".to_string());
        f3vs.push("            Ok(buf)".to_string());
        f3vs.push("        },".to_string());
    }

    let enumstr = vs.join("\n");
    let salt = rand::thread_rng().gen_range(0..645752165);
    let md5str = format!("{}{}", enumstr, salt);
    let version = md5::compute(&md5str);
    let version = format!("const PTO_VERSION: &str = \"{:x}\";", version);
    write_line(&mut file, &version)?;

    // function 1
    write_line(
        &mut file,
        r#"
pub fn is_proto_version(vers: &str) -> bool {
    PTO_VERSION.eq(vers)
}

"#,
    )?;

    let enumstr = format!(
        r#"#[derive(Debug)]
pub enum ProtoType {{
{}
}}
"#,
        enumstr
    );
    write_line(&mut file, &enumstr)?;

    // function parse_proto
    let fnstr = f2vs.join("\n");
    let f2 = format!(
        r#"
pub fn parse_proto(proto_id: u32,buf: &[u8], start_pos: usize, end_pos: usize) -> ::core::result::Result<ProtoType,crate::Error> {{
    match proto_id {{
{}
        _ => Err(crate::Error::Message(format!("[allptos.parse_proto]: failed, proto_id={{}}",proto_id)))
    }}
}}"#,
        fnstr
    );
    write_line(&mut file, &f2)?;

    // function serialize
    let fnstr = f3vs.join("\n");
    let f3 = format!(
        r#"
pub fn serialize(pto: ProtoType) -> ::core::result::Result<Vec<u8>, crate::Error> {{
    match pto {{
{}
    }}
}}"#,
        fnstr
    );
    write_line(&mut file, &f3)?;

    //tail
    write_line(&mut file, "")?;
    Ok(())
}

fn write_file_header(file: &mut File) -> Result<()> {
    file.write_all(b"//this file is automatically generated by protos. please do not edit.\n")?;
    let str = r#"#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]"#
        .to_string();
    file.write_fmt(format_args!("{}\n\n", &str))?;
    Ok(())
}

fn write_struct(file: &mut File, struct_name: &str, body: &str) -> Result<()> {
    file.write_fmt(format_args!(
        r#"
#[derive(Debug,Default)]
pub struct {} {{
{}
}}
"#,
        struct_name, body
    ))?;
    Ok(())
}

fn write_line(file: &mut File, line: &str) -> Result<()> {
    file.write_all(line.as_bytes())?;
    Ok(())
}

fn write_imp_write_for_struct(file: &mut File, struct_name: &str, body: &str) -> Result<()> {
    file.write_fmt(format_args!(
        "impl MsgWrite for {} {{\n{}\n}}",
        struct_name, body
    ))?;
    Ok(())
}

fn write_imp_read_for_struct(file: &mut File, struct_name: &str, body: &str) -> Result<()> {
    file.write_fmt(format_args!(
        "impl MsgRead for {} {{\n{}\n}}",
        struct_name, body
    ))?;
    Ok(())
}

fn write_imp_struct_with_random_default(
    file: &mut File,
    struct_name: &str,
    body: &str,
) -> Result<()> {
    file.write_fmt(format_args!(
        "impl {} {{\n    pub fn default_with_random_value() -> Self {{\n\t\tlet mut msg = Self::default();\n{}\n\t\tmsg\n\t}}\n}}\n",
        struct_name, body
    ))?;
    Ok(())
}

// every protocol should have its own test function.
fn generate_test_func(outdir: &str, pto: &Rc<RefCell<Pto>>) -> Result<()> {
    let entity_name = pto.borrow_mut().name.clone();
    let fname = format!("{}/../../tests/test_{}.rs", outdir, entity_name);
    let mut file = File::create(fname).unwrap();

    // header
    let header = r#"//this file is automatically generated by protos. please do not edit. 
use proto::{MsgWrite, MsgRead};
extern crate proto;
"#;
    file.write_fmt(format_args!("{}\n\n", header))?;

    let str = format!(
        r#"
#[test]
fn testfunc_{0}() {{
    let {0} = proto::{0}::{0}::default_with_random_value();
    //println!("{{:?}}", {0});
    let msglen = {0}.size();
    println!("{0}.size: {{}}", msglen);

    let mut buf = Vec::with_capacity(msglen);
    let mut w = proto::BytesWriter::new(&mut buf);
    {0}.write(&mut w).unwrap();
    assert!(buf.len() == msglen);
    assert!(buf.capacity() == msglen);
    println!("{0} into buf: successful, objsize: {{}}",msglen);

    let mut r = proto::BytesReader::new(0,buf.len());
    let s2 = proto::{0}::{0}::read(&mut r, &buf).unwrap();
    let msglen2 = s2.size();
    assert!(msglen == msglen2);
    let mut buf2 = Vec::with_capacity(msglen2);
    let mut w2 = proto::BytesWriter::new(&mut buf2);
    s2.write(&mut w2).unwrap();
    assert_eq!(buf,buf2);
    println!("{0} from buf: successful");
}}"#,
        entity_name
    );

    file.write_fmt(format_args!("{}\n", str))?;

    Ok(())
}
