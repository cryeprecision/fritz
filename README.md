# fritz-log-parser

## Timezone

Need to set the `TZ` docker container environment variable to the same timezone as the FRITZ!Box.
Logs fetched from the FRITZ!Box are assumed to be in the `Local` Timezone.

For example, if the timezone `Europe/Berlin` is set in the FRITZ!Box, `TZ=Europe/Berlin` should be
passed to the docker container. This can be confirmed by running `docker exec -it <CONTAINER-NAME> date`.

Before inserting logs into the database, they are converted from `Local` to `Utc` time and when
fetching logs from the database they are converted from `Utc` to `Local` time.

## Commands

- Reset the database
  - `cargo sqlx database reset --source ./data/migrations/`
- Create the database
  - `cargo sqlx database setup --source ./data/migrations/ --sqlite-create-db-wal false`

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

## Logic

### Login

- Response has block-time greater zero
  - Either someone else entered a wrong password and we just have to wait
  - Or our password is incorrect and we're the reason for the block time

### Sorting (`I`)

- The **FRITZ!Box** outputs logs sorted from **new to old**.
- Database stores logs from old to new. If two entries have the
  same timestamp, the one with the **greater row id** is the **newer** one.

Multiple log entries can have the same timestamp, e.g.

```text
(A): 2023-04-14T10:07:39+02:00 [1] -- in db
(B): 2023-04-14T10:07:39+02:00 [2] -- in db
(C): 2023-04-14T10:07:39+02:00 [3] -- new
(D): 2023-04-14T10:07:39+02:00 [4] -- new
```

In order to keep the database in sync with the FRITZ!Box logs,
old logs cannot be inserted retroactively. Because if there are logs `[a, b, c, d]`
and the database already contains logs `[c, d]` you cannot insert logs `[a, b]`
anymore, because if log `b` and `c` have the same timestamp, we cannot ensure original order.

### Sorting (`II`)

Because of the problem stated above, it is theoretically possible that some of the entries
with the same timestamp will never get added to the database. Altough this is very unlikely.
