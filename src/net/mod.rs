use std::{error::Error, rc::Rc};

use ctru::services::soc::Soc;

use self::mbedtls::{Ssl, Config, Net};

mod mbedtls;

pub fn connect<'a>(soc: &'a Soc, hostname: &str) -> Result<Ssl<'a>, Box<dyn Error>> {
    let mut conf = Config::new();
    conf.tls_client_defaults()?;
    conf.auth_mode_optional();
    conf.set_rng();
    let mut net = Net::new(soc);
    net.tcp_connect(hostname, 443)?;
    let mut ssl = Ssl::new(Rc::pin(conf), Box::pin(net))?;
    ssl.set_hostname(hostname)?;
    ssl.handshake()?;
    Ok(ssl)
}
