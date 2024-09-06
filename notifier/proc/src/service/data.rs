use crate::proc_meta_parser::Parser;
use linked_hash_map::LinkedHashMap;
use quote::ToTokens;
use syn::{spanned::Spanned, Attribute, Error, Expr, Ident, LitBool, Path};

#[derive(Debug)]
pub struct PublisherData {
    pub protected: Option<LitBool>,
}

#[derive(Debug)]
pub enum SubscriberType {
    PubSub,
    Rpc,
}

#[derive(Debug)]
pub struct SubscriberData {
    pub count: Expr,
    pub name: Ident,
    pub mixed: Option<(Path, Expr)>,
    pub ty: SubscriberType,
}

#[derive(Debug)]
pub struct Data {
    pub notifier: Path,
    pub count: Option<Expr>,
    pub rpc: Option<Expr>,
    pub publishers: LinkedHashMap<Path, PublisherData>,
    pub subscribers: LinkedHashMap<Path, SubscriberData>,
}

impl Data {
    pub fn new(attrs: &Vec<Attribute>) -> Result<Self, Error> {
        let mut this = Self {
            count: None,
            rpc: None,
            notifier: syn::parse_quote!(Self),
            publishers: Default::default(),
            subscribers: Default::default(),
        };

        for attr in attrs {
            if attr.path().is_ident("notifier_service") {
                let mut parser = Parser::new(["notifier", "count", "rpc"], attr.span());
                attr.parse_nested_meta(|meta| parser.parse(meta))?;

                this.notifier = parser.get("notifier")?;
                this.count = parser.get("count").ok();
                this.rpc = parser.get("rpc").ok();
            } else if attr.path().is_ident("notifier_publisher") {
                let mut parser = Parser::new(["event", "protected"], attr.span());
                attr.parse_nested_meta(|meta| parser.parse(meta))?;

                let data = PublisherData {
                    protected: parser.get("protected").ok(),
                };

                let key = parser.get::<Path>("event")?;
                let span = key.span();
                let event = key.to_token_stream().to_string();
                if this.publishers.insert(key, data).is_some() {
                    return Err(Error::new(
                        span,
                        format!("Event {event} already publishing"),
                    ));
                }
            } else if attr.path().is_ident("notifier_subscriber") {
                let mut parser =
                    Parser::new(["event", "count", "mixer", "mix_mapper"], attr.span());
                attr.parse_nested_meta(|meta| parser.parse(meta))?;

                let mixer = parser.get("mixer");
                let mix_mapper = parser.get("mix_mapper");
                let mixed = match (mixer, mix_mapper) {
                    (Err(_), Err(_)) => None,
                    (Ok(mixer), Ok(mix_mapper)) => Some((mixer, mix_mapper)),
                    (_, Err(err)) | (Err(err), _) => return Err(err),
                };

                let key = parser.get::<Path>("event")?;
                let data = SubscriberData {
                    count: parser.get("count")?,
                    ty: SubscriberType::PubSub,
                    name: Ident::new(&format!("_{}", this.subscribers.len()), key.span()),
                    mixed,
                };

                let span = key.span();
                let event = key.to_token_stream().to_string();
                if this.subscribers.insert(key, data).is_some() {
                    return Err(Error::new(
                        span,
                        format!("Already subscribed on event {event}"),
                    ));
                }
            } else if attr.path().is_ident("notifier_rpc_subscriber") {
                let mut parser = Parser::new(["service", "count"], attr.span());
                attr.parse_nested_meta(|meta| parser.parse(meta))?;

                let key = parser.get::<Path>("service")?;
                let data = SubscriberData {
                    count: parser.get("count")?,
                    ty: SubscriberType::Rpc,
                    name: Ident::new(&format!("_{}", this.subscribers.len()), key.span()),
                    mixed: None,
                };

                let span = key.span();
                let event = key.to_token_stream().to_string();
                if this.subscribers.insert(key, data).is_some() {
                    return Err(Error::new(
                        span,
                        format!("Already subscribed on event {event}"),
                    ));
                }
            }
        }

        Ok(this)
    }
}
