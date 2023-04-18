# fritz-log-parser

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

### Table structures

```sql
-- TODO
```
