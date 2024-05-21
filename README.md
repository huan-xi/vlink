兼容标准WireGuard协议的异地组网工具
对标tailscale

服务器管理客户端秘钥达到0配置

多种两节方式
标准WireGuard协议子支持udp 协议,

对于标准的WireGuard客户端，需要互联, 要么一方有公网，要么走服务器udp代理

udpForwarder
============
udp 转发器, 当两个目标需要

# 组件

vlinkd
> 服务端管理客户端秘钥，提供连接信息,生成vlink-tun
>

## 不创建tun,只提供socks5 代理,不会改变系统路由表

## 中继服务器链路合并

# 端点连接协议

1. udp(兼容标准WireGuard)
2. tcp
3. ws
4. nat1_穿透
5. auto
6. udpForwarder-frp 通过frp穿透udp端口直连



vlinkd
与服务器建立连接，管理一个网络

1. 默认两端udp端口可达,A向B发送握手请求, B收到A请求向A发送握手响应
2. 一端(B)tcp,

参考项目
=======
https://github.com/lbl8603/vnt
https://github.com/juanfont/headscale
https://github.com/tailscale/tailscale
https://github.com/cloudflare/boringtun
https://github.com/zarvd/wiretun
