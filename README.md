# fritz-log-parser

## Contributing

Contributions are welcome! Just open a pull request and I'll take a look as soon
as I see it. Please make sure the following requrements are met:

- If you change the database layout
  - Try to do so with a new migration using `cargo sqlx migrate add --source ./data/migrations/ <MIGRATION-NAME>` to not break existing databases.
  - Run `cargo sqlx prepare` so offline compilation still works.
- Run `cargo clippy` and fix the warnings.
- Make sure the code still compiles and builds as a docker container.

**Note**: The code in `/src/db/connection.rs`, specifically `Database::append_new_logs` tries to handle the weird way logs are saved in the FRITZ!Box. If you know a way to access the **raw** logs, please open an issue.

## Environment variables

See **Deploy** section for an example configuration.

- `DATABASE_URL`: See [`sqlx`](https://docs.rs/sqlx/latest/sqlx/), must point to a SQLite database. Also see [github.com/launchbadge/sqlx/issues/1114#issuecomment-827815038](https://github.com/launchbadge/sqlx/issues/1114#issuecomment-827815038).
- `FRITZBOX_DOMAIN`: Domain part of the FRITZ!Box URL. (e.g. `192.168.178.1` or `fritz.box`)
- `FRITZBOX_USERNAME`: Username of the user this service should use.
- `FRITZBOX_PASSWORD`: Password of the user this service should use.
- `FRITZBOX_REFRESH_PAUSE_SECONDS`: How many seconds to wait between fetching logs.
- `FRITZBOX_ROOT_CERT_PATH`: If you're using a custom certificate for the FRITZ!Box, you can set this path to point to the certificate of the CA (Certificate Authority) the certificate has been signed with. Otherwise all certificates will be accepted.
- `FRITZBOX_SAVE_RESPONSE`: Whether to save responses received from the FRITZ!Box to a file, can be `true` or `false` or omitted.
- `FRITZBOX_SAVE_RESPONSE_PATH`: A path to the folder where the received responses will be saved to, each response will be saved to its own file within this folder.

## Deploy

### 1. Build the image

I run a local registry and use a script similar to this

```sh
#!/bin/bash
set -e

IMAGE_NAME=fritz-log-parser
REGISTRY=your.registry.com

docker build -t $IMAGE_NAME .
docker tag $IMAGE_NAME $REGISTRY/$IMAGE_NAME
docker push $REGISTRY/$IMAGE_NAME
```

### 2. Deploy the image

I use a `docker-compose.yml` similar to this

```yaml
name: fritz
services:
  fritz:
    image: your.registry.com/fritz-log-parser
    restart: unless-stopped
    environment:
      TZ: Europe/Berlin
      DATABASE_URL: sqlite:///opt/fritz/database/logs.db3
      FRITZBOX_USERNAME: fritz6969
      FRITZBOX_PASSWORD: pASSword
      FRITZBOX_DOMAIN: 192.168.178.1
      FRITZBOX_ROOT_CERT_PATH: /opt/fritz/certificates/cert.pem
      FRITZBOX_REFRESH_PAUSE_SECONDS: 300
    volumes:
      - ./container-data/fritz/:/opt/fritz/
```

You can deploy it with `docker compose -f docker-compose.yml up -d` where `-f`
specifies the compose file to use and `-d` starts the container in the
background.

### 3. Check the logs

If you used the name from above you can check the logs with
`docker logs -f fritz-fritz-1` where `-f` specifies to follow the log output.

The logs should be similar to this

```txt
23:27:48 [WARN] couldn't load .env file: Io(Custom { kind: NotFound, error: "path not found" })
23:27:54 [INFO] login-challenge request to https://<OMITTED>/login_sid.lua?version=2 (GET - 200) took 5501ms (session-id: None)
23:27:55 [INFO] login-response request to https://<OMITTED>/login_sid.lua?version=2 (POST - 200) took 775ms (session-id: None)
23:27:55 [INFO] check-session-id request to https://<OMITTED>/login_sid.lua?version=2 (POST - 200) took 423ms (session-id: Some(<OMITTED>))
23:27:56 [INFO] logs request to https://<OMITTED>/data.lua (POST - 200) took 1122ms (session-id: Some(<OMITTED>))
23:27:56 [INFO] upserted 9 logs
```

### 4. Analyse the database

Fetch the database (see **Commands** section), open it using
[DataGrip](https://www.jetbrains.com/datagrip/),
[DB Browser for SQLite](https://sqlitebrowser.org/) or anything else that works
for you and run some queries.

## Timezones

Need to set the `TZ` docker container environment variable to the same timezone
as the FRITZ!Box because logs fetched from the FRITZ!Box are assumed to be in
the `chrono::Local` Timezone.

For example, if the FRITZ!Box has the timezone `Europe/Berlin` set,
`TZ=Europe/Berlin` should be passed to the docker container. This can be
confirmed by running `docker exec -it <CONTAINER-NAME> date`.

**Note**: Before inserting logs into the database, they are converted from
`chrono::Local` to `chrono::Utc` and when fetching logs from the database, they
are assumed to be in `chrono::Utc` and will be converted to `chrono::Local`.

## Commands

- Create the database
  - `cargo sqlx database setup --source ./data/migrations/ --sqlite-create-db-wal false`
- Reset the database
  - `cargo sqlx database reset --source ./data/migrations/ --sqlite-create-db-wal false`
- Fetch the current database
  - `scp <USER>@<HOST>:<PATH-TO-DB> <SAVE-PATH>`

## Queries

- Convert timestamps to readable localtime
  - `DATETIME(FLOOR(<FIELD-NAME> / 1000), 'unixepoch', 'localtime')`
  - Divide by `1000` because timestamps have millisecond precision

## Resources

- GitHub
  - [github.com/arctic-alpaca/fritz_box_tr064_igd_api_files_generator](https://github.com/arctic-alpaca/fritz_box_tr064_igd_api_files_generator)
  - [github.com/kbr/fritzconnection](https://github.com/kbr/fritzconnection)
- AVM
  - [Session-ID_deutsch_13Nov18.pdf](https://avm.de/fileadmin/user_upload/Global/Service/Schnittstellen/Session-ID_deutsch_13Nov18.pdf)
  - [AVM Technical Note - Session ID_deutsch - Nov2020.pdf](https://avm.de/fileadmin/user_upload/Global/Service/Schnittstellen/AVM%20Technical%20Note%20-%20Session%20ID_deutsch%20-%20Nov2020.pdf)
- Misc
  - [rust.helpful.codes/tutorials/reqwest/Sending-Form-Data-and-Uploading-Files-with-Reqwest](https://rust.helpful.codes/tutorials/reqwest/Sending-Form-Data-and-Uploading-Files-with-Reqwest/)
  - [cryptii.com/pipes/integer-encoder](https://cryptii.com/pipes/integer-encoder)
  - [entrust.com/how-do-i-convert-my-pem-certificate-to-a-der-certificate-format](https://www.entrust.com/knowledgebase/ssl/how-do-i-convert-my-pem-certificate-to-a-der-certificate-format)
