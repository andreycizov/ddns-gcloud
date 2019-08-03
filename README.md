**ddns-gcloud** is a utility to update your current IP in Google Cloud. It's useful for those
those who can't get a static IP for their home PC and thus need it to be regularly updated.

It's an all-in-one solution that also polls the IP from https://icanhazip.com.

The design is specifically optimised to reduce the costs associated with constantly
using get or mutation operations in Cloud DNS. Basically, I am currently paying about ~$0.10
for two zones managed by this tool.

If you're looking to cross-compile this tool for another platform - please look at the
[cross-compilation script](./build-armhf) I used for Raspberry PI.

## Help

```text
>> ddns-cloud --help
ddns-gcloud 0.1.0
Andrey Cizov <acizov@gmail.com>

USAGE:
    ddns-gcloud [FLAGS] [OPTIONS] -p <project_id> -n <record_name> -z <zone_name> <SUBCOMMAND>

FLAGS:
    -I               Only update the latest IP setting, skip the check against previous value
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -i <cache>              Location of the file that holds the latest IP setting (e.g. ip.txt)
    -c <change_id>          Register as this change id
    -p <project_id>         Project ID in Google Cloud (e.g. "horrendous-student-23452")
    -n <record_name>        Google Cloud DNS zone record set name (e.g. "myhome.example.com.")
    -l <record_ttl>         Google Cloud DNS zone record set TTL [default: 1]
    -t <record_type>        Google Cloud DNS zone record set type [default: A]
    -s <secrets>            Location of the file containing the authentication secrets for the app (JSON downloaded from
                            Google Cloud) [default: secrets.json]
    -k <tokens>             Location of the file that would hold all of the tokens (this will be created if empty)
                            [default: tokens.json]
    -z <zone_name>          Google Cloud DNS zone ID (e.g. "example")

SUBCOMMANDS:
    auth    Create the authentication files
    help    Prints this message or the help of the given subcommand(s)
    ip      Show the loaded IP
    set     Sets the IP to whatever had been acquired
```

## Set up

 1. You would need to create a project in Google Cloud. That project will have an ID that is used
    for `-p` option.
 1. You would then need to create a set of credentials for that project. You can download them as a JSON file that you
    can then provide with `-s` option to the utility.
 2. Then, you would need to create a managed zone in Google Cloud. Each zone has a simple alphanumeric name,
    which also doubles as it's ID - this is passed as `-z` option.

### Usage

Supply all of the arguments described above and additional ones supplied in the help.

#### To set an IP

```bash
> ddns-gcloud -p horrendous-student-23452 -z example -n myhome.example.com. set
> $?
0
```

```bash
> ddns-gcloud -p horrendous-student-23452 -z example -n myhome.example.com. set
not authenticated. please use `auth` command to authenticate
> $?
255
```

#### To get an IP

```bash
> ddns-gcloud -p horrendous-student-23452 -z example -n myhome.example.com. ip
255.123.43.32
```

#### To authenticate

```bash
> ddns-gcloud -p horrendous-student-23452 -z example -n myhome.example.com. auth
Please direct your browser to https://accounts.google.com/o/oauth2/auth?<LONG_URL>, follow the instructions and enter the code displayed here:
```


## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
         http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the
work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.