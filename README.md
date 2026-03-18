
# File Express

*FileExpres* (`fexpress`) is a simple and secure peer-to-peer file transfer CLI written in Rust.  
It allows two peers in the same LAN to send files or directories directly over the network using encrypted connections.

The program uses a TLS-based P2P authentication system where peers verify each other using **certificate fingerprints**.

# Features

- 🔐 **Encrypted peer-to-peer connections**
- 🪪 **Certificate fingerprint verification**
- 📁 **Send files or entire directories**
- 📦 **Automatic directory compression**
- 🧩 **Cross-platform (Linux, macOS, Windows)**
- ⚡ **Asynchronous networking using Tokio**


## Usage / Examples

### Generate Identity & Hash

Generates a self-signed certificate and private key required to run **fexpress**.  
It also prints a hash that acts like a “public key” for mTLS.  
This hash should be shared with the other peer to establish a secure connection.  

Running this command again will generate a **new hash**, invalidating the previous one.

```bash
fexpress generate
```
***Get Hash***

You can run `fexpress hash` to get the hash again without changing it.

***Listen (Receiver)***

Start a receiver to wait for incoming files:
```bash
fexpress listen --port 8080 --path /path/to/download --expected-client-hash <hash>
```

***Send (Sender)***

Send a file or directory to a receiver:
```bash
fexpress send --path /path/to/file --to 192.168.1.67:8080 --expected-server-hash <hash>
```


## License

[MIT](https://github.com/Bicheka/file-express/blob/main/LICENSE)

