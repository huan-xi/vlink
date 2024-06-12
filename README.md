兼容标准WireGuard协议的异地组网工具对标tailscale

服务器管理客户端秘钥达到0配置

多种连接方式
标准WireGuard协议只支持udp, 对于标准的WireGuard客户端，需要互联, 要么一方有公网，要么走服务器udp代理

丰富的扩展协议

1. udp(兼容标准WireGuard)
2. tcp(保持连接)
3. ws(可以nginx反向代理)
4. nat1穿透(tcp,udp)
5. 动态ip(dip),当公网ip动态变化时第一时间通知节点
6. udpForwarder-frp 通过frp穿透udp端口直连
7. tailscale derp (使用tailscale中继协议)

## 客户端端口转发

### 功能介绍

> 内网中只有主机A安装了一个vlinkd,想暴露主机B的80端口, 可以通过vlinkd 设置80端口转发到主机B 时得访问主机A的80端口等于访问主机B的80端口

### 使用场景

> 开发中 rds 数据库未暴露公网端口，只需要ecs上安装vlinkd 设置端口转发，则可以通过vlinkd访问rds数据库

## nat1_穿透

### 功能介绍

> tcp穿透

### 使用场景

> 内网群晖想映射到公网，让外网访问群晖

## acl 访问控制(未实现)

> 通过acl控制访问权限


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
