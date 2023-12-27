# File Hosting Service
---
#### Prerequisites
- Have `cargo` installed (if not using Docker)
---
#### Configuration:
- Update the .env.example files and rename them to .env
---
## Server
Run the command: ```cargo run --release``` to initiate the server

> For compilation, ```cargo build --release```

#### Env Variables 
| Name | Description |
| :--- | :----: |
| Port | The server port |
| Upload_dir | Uploaded files will be placed here |
| Access_key | Auth key used when uploading files (you have to create one) |

#### Using Docker
In order to use the containerized version of the server, you can use [`docker-compose.yml`](/server/docker-compose.yml) (Changing the .env file will not do anything)

1. Edit the `docker-compose.yml` file to set:

 - **HOST_PORT:** The port on the host which will expose the server
- **ACCESS_KEY:** Auth key used when uploading files
- **HOST_DIRECTORY:** Directory on host that will be bound to the container's upload directory (will let you view uploads)

2. Run `docker-compose up -d` and wait for the service to start

> NOTE: Do **NOT** change anything not in [] unless you alter the `Dockerfile` as well
---
## Client
The client is standard CLI tool that you can use to upload files with. For more help, you can run:

- `cargo run -- -h` if using cargo
- `./client -h` if using a binary (compilation steps are the same as server)

> For directory uploads, the uploaded links for each file will be recorded in the `records.json` file (or whatever the user selects with the `-r` arguement)

#### Env Variables
| Name | Description |
| :--- | :----: |
| Base_url | Base Url of file server (https://example.com) |
| Access_key | Auth key used when uploading files (Needs to be the same as server's) |
