# Reddy
Reddy is a small server written in Rust that acts as an HTTP wrapper around Redis. For example, if an `HTTP GET /hello` is received by the server, it will then get the `hello` key from Redis and return its value. You can also use `POST` to set a key.

Additionally, this server also performs a simple form of _hash-based sharding_ over the Redis keys, providing a simple mechanism for balancing keys across multiple separate Redis servers.

Reddy is part of the [ansible-demo](https://github.com/ThomasMiz/ansible-demo) project.

## Compiling
This project is intended to be built with [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html), which is installed with [rustup](https://rustup.rs/):

On GNU/Linux, this is as simple ad running the following command:
```bash
$ curl https://sh.rustup.rs -sSf | sh
```

On windows, the [exe installer](https://win.rustup.rs/x86_64) is recommended instead.

You can check that cargo is installed with:
```bash
$ cargo --version
cargo 1.78.0 (54d8815d0 2024-03-26)
```

You may need to restart your command terminal for the installation to take effect.

With cargo installed, let's clone the project from GitHub and built the project with:
```bash
$ git clone https://github.com/ThomasMiz/reddy.git
$ cd reddy
$ cargo build --release
```

This will automatically download all the dependencies and compile the project into a single executable, which will be located at `./target/release/reddy`.

## Usage
If you run the reddy executable immediately after compiling, you will notice an error indicating that no `.env` file was found. Reddy expects this plaintext file to be in its working directory (typically the same folder as the executable) which configures three things; the bind address, the instance name, and the list of redis hosts:

```text
LISTEN_AT=0.0.0.0:8080
INSTANCE_NAME=reddy-server1
REDIS_HOSTS=redis://127.0.0.1:6379;redis://127.0.0.1:6380
```

Let's go over each of these:
* The `LISTEN_AT` is the IP address in which to listen for incoming HTTP clients.
* The `INSTANCE_NAME` is a meaningless string that gets added as an `X-Reddy-Instance-Name` header to all HTTP responses. This can be used to identify which Reddy instance responded to a request when multiple instances are running transparently behind a load balancer.
* The `REDIS_HOSTS` indicates the list of Redis servers to connect to, with hosts separated by colons ';'. The order in which these hosts are specified matters, as this affects how sharding is performed. If multiple Reddy instances are connecting to the same Redis servers, you must specify the REDIS_HOSTS in the same order.

To create your `.env` file, copy the `.env-example` at the root of this project, place it next to the Reddy executable, and name it `.env`. Then open the file with any plain text editor and apply your desired configuration.

Starting the server is as simple as executing the reddy executable:
```bash
$ ./reddy
```

When the server is started, it will check for conectivity with each of the specified Redis servers by setting and getting the "test" key.

## Additional information
Apart from the previously mentioned `X-Reddy-Instance-Name` header, Reddy also adds an `X-Redis-Instance-Index` header to the HTTP responses indicating the index number of the Redis instance accessed.
