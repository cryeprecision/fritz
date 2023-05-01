# fritz-log-parser

## Problems

Can't use [Rustls](https://github.com/rustls/rustls) because it doesn't accept the FRITZ!Box root certificate.

- [stackoverflow.com/70187309#comment124072621_70187309](https://stackoverflow.com/questions/70187309/tls-error-using-reqwest-a-ca-certificate-is-being-used-as-an-end-entity-certifi#comment124072621_70187309)

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

## Structure

### Log entry

A log entry holds the following data

- Date and time
- The log message
- The category of the log message
  - E.g. Internet, Phone, ...
- The ID of the log message
  - An unsigned integer

### Log message category

There are five categories for log messages

- Internet
- Phone
- System
- USB
- WLAN

## SqLite Database

```sql
CREATE TABLE logs(
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    message    VARCHAR NOT NULL,
    message_id INTEGER NOT NULL,
    category   INTEGER NOT NULL,
    logged_at  INTEGER NOT NULL
);

-- fast lookup of newest logs
CREATE INDEX IF NOT EXISTS
    logs_logged_at_index
ON logs (logged_at DESC);

-- the combination of `logged_at` and `message_id` must be unique
CREATE UNIQUE INDEX IF NOT EXISTS
    logs_unique_index
ON logs (logged_at DESC, message_id);
```

### Sorting (`I`)

There are some seemingly unnecessary `ORDER BY` directives when selecting rows,
but we want to make sure that the result is always ordered as they'd be in the original
FRITZ!Box log and the optimizer can always remove it if it's really obsolete.

- The FRITZ!Box outputs logs sorted from new to old.
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
