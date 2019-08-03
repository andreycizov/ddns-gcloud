use std::default::Default;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use structopt::StructOpt;

use hyper_rustls;
use google_dns1::{Change, Dns, ResourceRecordSetsListResponse, ResourceRecordSet, Scope};
use yup_oauth2::{Authenticator, FlowType, ConsoleApplicationSecret, DiskTokenStorage, AuthenticatorDelegate, DefaultAuthenticatorDelegate};

use hyper::{Client, Url};
use hyper::net::HttpsConnector;
use hyper::status::StatusCode;
use std::str::from_utf8;
use hyper::method::Method;
use std::process::exit;

#[derive(Debug, StructOpt)]
enum Command {
    #[structopt(name = "set")]
    /// Sets the IP to whatever had been acquired
    Set,
    #[structopt(name = "auth")]
    /// Create the authentication files
    Authenticate,
    #[structopt(name = "ip")]
    /// Show the loaded IP
    IP,
}

#[derive(Debug, StructOpt)]
#[structopt(name = "ddns-gcloud")]
struct Opt {
    #[structopt(short = "s", default_value = "secrets.json")]
    /// Location of the file containing the authentication secrets for the app (JSON downloaded from Google Cloud)
    secrets: String,
    #[structopt(short = "k", default_value = "tokens.json")]
    /// Location of the file that would hold all of the tokens (this will be created if empty)
    tokens: String,

    #[structopt(short = "c")]
    /// Register as this change id
    change_id: Option<String>,

    #[structopt(short = "i")]
    /// Location of the file that holds the latest IP setting (e.g. ip.txt)
    cache: Option<String>,

    #[structopt(short = "I")]
    /// Only update the latest IP setting, skip the check against previous value
    cache_only_update: bool,

    #[structopt(short = "p")]
    /// Project ID in Google Cloud (e.g. "horrendous-student-23452")
    project_id: String,

    #[structopt(short = "z")]
    /// Google Cloud DNS zone ID (e.g. "example")
    zone_name: String,

    #[structopt(short = "n")]
    /// Google Cloud DNS zone record set name (e.g. "www.example.com.")
    record_name: String,

    #[structopt(short = "l", default_value = "1")]
    /// Google Cloud DNS zone record set TTL
    record_ttl: i32,

    #[structopt(short = "t", default_value = "A")]
    /// Google Cloud DNS zone record set type
    record_type: String,

    #[structopt(subcommand)]
    command: Command,
}

pub struct OptAuthenticatorDelegate {
    inner: DefaultAuthenticatorDelegate,
    can_interact: bool
}

impl OptAuthenticatorDelegate {
    fn from_opts(opt: &Opt) -> Self {
        OptAuthenticatorDelegate {
            inner: DefaultAuthenticatorDelegate,
            can_interact: if let Command::Authenticate = opt.command {
                true
            } else {
                false
            }
        }
    }
}

impl AuthenticatorDelegate for OptAuthenticatorDelegate {
    fn present_user_url(&mut self, url: &String, need_code: bool) -> Option<String> {
        if self.can_interact {
            self.inner.present_user_url(url, need_code)
        } else {
            eprintln!("not authenticated. please use `auth` command to authenticate");
            exit(-1)
        }
    }
}


fn main() {
    let opt: Opt = Opt::from_args();

    let (secrets, _is_web) = {
        let file = File::open(opt.secrets.clone());
        let mut file = file.expect("secrets file not found");
        let mut content = String::new();

        file.read_to_string(&mut content).expect("secrets file can't be read");

        let secrets: ConsoleApplicationSecret =
            serde_json::from_str::<ConsoleApplicationSecret>(&content).expect("not a good file");

        if let Some(x) = secrets.installed {
            (x, false)
        } else if let Some(x) = secrets.web {
            (x, true)
        } else {
            panic!("authentication file has no available components")
        }
    };

    let storage = {
        DiskTokenStorage::new(&opt.tokens).expect("null ifle")
    };

    let auth = {
        let mut auth = Authenticator::new(
            &secrets,
            OptAuthenticatorDelegate::from_opts(&opt),
            Client::with_connector(HttpsConnector::new(hyper_rustls::TlsClient::new())),
            storage,
            Some(FlowType::InstalledInteractive)
        );

        // make sure that all of these scopes are authorized
        let scopes: Vec<String> = vec![
            Scope::NdevClouddnReadonly.as_ref().to_string(),
            Scope::CloudPlatform.as_ref().to_string(),
        ];

        use yup_oauth2::GetToken;

        auth.token(&scopes).expect("must be authenticated for the following scopes");

        auth
    };

    if let Command::Authenticate = opt.command {
        return;
    }

    let ip: String = {
        let client = Client::with_connector(HttpsConnector::new(hyper_rustls::TlsClient::new()));
        let url: Url = "https://icanhazip.com".parse().expect("must be a correct URL address");

        let builder = client.request(Method::Get, url);

        let mut res = builder.send().expect("must return something back");


        if let StatusCode::Ok = res.status {
            let mut buf = Vec::with_capacity(256);
            let _size = res.read_to_end(&mut buf).expect("must successfully read");

            let ip: String = from_utf8(&buf).expect("could not parse the response from the server").into();
            let ip = ip.trim();

            use std::net::Ipv4Addr;

            let _ip2: Ipv4Addr = ip.parse().expect(&format!("must be valid ipv4 addr: `{}`", ip));

            ip.to_string()
        } else {
            panic!("{}", res.status);
        }
    };

    if let Command::IP = opt.command {
        println!("{}", ip);
        return;
    }

    if let Some(value) = &opt.cache {
        use std::io::ErrorKind;

        match File::open(value) {
            Ok(mut x) => {
                let mut content = String::new();

                x.read_to_string(&mut content).expect("must be able to read");

                let content = content.trim();

                if content == ip && !opt.cache_only_update {
                    eprintln!("skipping due to cache");
                    return;
                }
            }
            Err(x) => match x.kind() {
                ErrorKind::NotFound => {}
                _ => {
                    panic!("cache file error {}", x)
                }
            }
        }

        let mut opener = OpenOptions::new();
        opener.create(true);
        opener.write(true);
        opener.truncate(true);

        let mut cache = opener.open(value).expect("must be able to open cache file");

        cache.write(ip.as_bytes()).expect("can't write to file");
        cache.write("\n".as_bytes()).expect("can't write to file");
    }

    let hub = Dns::new(
        Client::with_connector(HttpsConnector::new(hyper_rustls::TlsClient::new())),
        auth,
    );

    let record_set: Option<ResourceRecordSet> = {
        let name = &opt.record_name;
        let type_ = &opt.record_type;

        let result_list = hub.resource_record_sets().list(
            &opt.project_id, &opt.zone_name,
        ).doit();

        let result_list: ResourceRecordSetsListResponse = result_list.expect("must be ok").1;
        if let Some(rrsets) = &result_list.rrsets {
            let mut iterator = rrsets.iter();
            let ret: Option<&ResourceRecordSet> = loop {
                if let Some(rrset) = iterator.next() {
                    if let Some(rrname) = &rrset.name {
                        if let Some(rrtype) = &rrset.type_ {
                            if rrname == name && rrtype == type_ {
                                break Some(rrset);
                            }
                        }
                    }
                } else {
                    break None;
                }
            };
            let ret: Option<ResourceRecordSet> = ret.cloned();

            ret
        } else {
            None
        }
    };

    let mut req = Change::default();

    if let Some(x) = record_set {
        req.deletions = Some(vec![x.clone()]);
    }

    req.additions = Some(vec![
        ResourceRecordSet {
            rrdatas: Some(vec![ip.into()]),
            kind: Some("dns#resourceRecordSet".into()),
            name: Some(opt.record_name.into()),
            ttl: Some(opt.record_ttl),
            type_: Some(opt.record_type.into()),
            signature_rrdatas: Some(vec![]),
        }
    ]);

    let mut result = hub.changes().create(req, &opt.project_id, &opt.zone_name);

    if let Some(x) = opt.change_id {
        result = result.client_operation_id(&x);
    }

    let result = result.doit();


    let _rr = result.expect("must succeed");
}
