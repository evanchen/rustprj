# rustprj
a proto type for game service.用rust写的游戏服务器原型(探索中).

执行服务:
1. 定义协议文件(简化过的protobuf格式),生成协议编解码代码: cargo run -p protogen
2. 配置db服务端口等,先启动一个实列: cargo run -p rengine --bin service
3. 配置游戏服务端口等,再启动一个实列: cargo run -p rengine --bin service

todo:
客户端机器人协议测试(集成测试)