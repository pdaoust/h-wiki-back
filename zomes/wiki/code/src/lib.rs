#![feature(proc_macro_hygiene)]
#[macro_use]
extern crate hdk;
extern crate hdk_proc_macros;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate holochain_json_derive;

use hdk::holochain_core_types::{
    dna::entry_types::Sharing,
    entry::Entry,
    // agent::AgentId, dna::entry_types::Sharing, entry::Entry, link::LinkMatch,
    link::LinkMatch,
};
use hdk::holochain_json_api::{error::JsonError, json::JsonString};
use hdk::holochain_persistence_api::cas::content::{Address, AddressableContent};
use hdk::prelude::Entry::App;
use hdk::{
    entry_definition::ValidatingEntryType,
    error::ZomeApiResult,
    // AGENT_ADDRESS,
    // AGENT_ADDRESS, AGENT_ID_STR,
};
use hdk_proc_macros::zome;
use holochain_anchors;
// see https://developer.holochain.org/api/0.0.25-alpha1/hdk/ for info on using the hdk library

// This is a sample zome that defines an entry type "MyEntry" that can be committed to the
// agent's chain via the exposed function create_my_entry

#[derive(Serialize, Deserialize, Debug, DefaultJson, Clone)]
pub struct User {
    username: String,
    agent_adress: Address,
}
#[derive(Serialize, Deserialize, Debug, DefaultJson, Clone)]
pub enum Content {
    Text(String),
    Binarys(Vec<u8>),
}
#[derive(Serialize, Deserialize, Debug, DefaultJson, Clone)]
pub struct Section {
    page_address: Option<Address>,
    r#type: String,
    content: String,
    rendered_content: String,
}
#[derive(Serialize, Deserialize, Debug, DefaultJson, Clone)]
pub struct Page {
    title: String,
    sections: Vec<Address>,
}
#[derive(Serialize, Deserialize, Debug, DefaultJson, Clone)]
pub struct HomePage {
    title: String,
    sections: Vec<Section>,
}
impl Page {
    pub fn initial(title: String) -> Page {
        Page {
            title,
            sections: vec![],
        }
    }

    pub fn from(title: String, sections: Vec<Address>) -> Page {
        Page { title, sections }
    }
    fn entry(self) -> Entry {
        App("wikiPage".into(), self.into())
    }
    fn update(self, address: Address) -> ZomeApiResult<Address> {
        let entry = App("wikiPage".into(), self.into());
        hdk::api::update_entry(entry, &address)
    }
}

impl Section {
    fn entry(self) -> Entry {
        App("pageElement".into(), self.into())
    }
    fn update(self, address: Address) -> ZomeApiResult<Address> {
        let entry = App("pageElement".into(), self.into());
        hdk::api::update_entry(entry, &address)
    }
}
#[zome]
mod wiki {
    #[init]
    fn init() {
        Ok(())
    }
    #[validate_agent]
    pub fn validate_agent(validation_data: EntryValidationData<AgentId>) {
        Ok(())
    }
    #[entry_def]
    fn anchor_def() -> ValidatingEntryType {
        holochain_anchors::anchor_definition()
    }
    #[entry_def]
    fn user_def() -> ValidatingEntryType {
        entry!(
            name: "user",
            description: "this is an entry representing some profile info for an agent",
            sharing: Sharing::Public,
            validation_package: || {
                hdk::ValidationPackageDefinition::Entry
            },
            validation: | _validation_data: hdk::EntryValidationData<User>| {
                Ok(())
            },
            links: [
                from!(
                    "%agent_id",
                    link_type: "agent->user",
                    validation_package: || {
                        hdk::ValidationPackageDefinition::Entry
                    },
                    validation: | _validation_data: hdk::LinkValidationData| {
                        Ok(())
                    }
                )
            ]
        )
    }
    #[entry_def]
    fn page_def() -> ValidatingEntryType {
        entry!(
            name: "wikiPage",
            description: "this is an entry representing some profile info for an agent",
            sharing: Sharing::Public,
            validation_package: || {
                hdk::ValidationPackageDefinition::Entry
            },
            validation: | _validation_data: hdk::EntryValidationData<Page>| {
                Ok(())
            },
            links: [
                from!(
                    holochain_anchors::ANCHOR_TYPE,
                    link_type: "anchor->Page",
                    validation_package: || {
                        hdk::ValidationPackageDefinition::Entry
                    },

                    validation: |_validation_data: hdk::LinkValidationData| {
                        Ok(())
                    }
                )
            ]

        )
    }
    #[entry_def]
    fn page_element_def() -> ValidatingEntryType {
        entry!(
            name: "pageElement",
            description: "this is an entry representing some profile info for an agent",
            sharing: Sharing::Public,
            validation_package: || {
                hdk::ValidationPackageDefinition::Entry
            },
            validation: | _validation_data: hdk::EntryValidationData<Section>| {
                Ok(())
            }
        )
    }
    // #[zome_fn("hc_public")]
    // fn create_user(user:User)->ZomeApiResult<Address>{
    //     let user_entry=Entry::App("user".into(),user.into());
    //     let anchor_address = holochain_anchors::create_anchor("model".into(), "soft-tail".into())?;
    //     let user_adress=hdk::utils::commit_and_link(&user_entry,&anchor_address,"anchor->user","")?;
    //     hdk::api::link_entries(&AGENT_ADDRESS,&user_adress,"agent->user","")?;
    //     Ok(user_adress)
    // }
    pub fn pages_anchor() -> ZomeApiResult<Address> {
        holochain_anchors::create_anchor("wiki_pages".into(), "all_pages".into())
    }
    #[zome_fn("hc_public")]
    fn create_page(title: String) -> ZomeApiResult<String> {
        create_page_if_non_existent(title.clone())?;
        Ok(title)
    }
    pub fn create_page_if_non_existent(title: String) -> ZomeApiResult<Address> {
        let address = Page::initial(title.clone()).entry().address();
        match hdk::get_entry(&address)? {
            None => {
                let page_anchor = pages_anchor()?;
                hdk::utils::commit_and_link(
                    &Page::initial(title).entry(),
                    &page_anchor,
                    "anchor->Page",
                    "",
                )
            }
            Some(_) => Ok(address),
        }
    }
    #[zome_fn("hc_public")]
    fn update_page(contents: Vec<Section>, title: String) -> ZomeApiResult<String> {
        let page_address = create_page_if_non_existent(title.clone())?;
        let sections: Vec<Address> = contents
            .into_iter()
            .map(|mut element| {
                element.page_address = Some(page_address.clone());
                hdk::api::commit_entry(&element.entry())
            })
            .filter_map(Result::ok)
            .collect();

        Page::from(title.clone(), sections).update(page_address)?;
        Ok(title)
    }
    #[zome_fn("hc_public")]
    fn get_page(title: String) -> ZomeApiResult<Page> {
        hdk::utils::get_as_type::<Page>(Page::initial(title).entry().address())
    }
    fn get_titles() -> ZomeApiResult<Vec<String>> {
        let anchor_address = pages_anchor()?;
        Ok(hdk::utils::get_links_and_load_type::<Page>(
            &anchor_address,
            LinkMatch::Exactly("anchor->Page".into()),
            LinkMatch::Any,
        )?
        .into_iter()
        .map(|page| page.title)
        .collect())
    }
    #[zome_fn("hc_public")]
    fn get_home_page() -> ZomeApiResult<HomePage> {
        let vec = get_titles()?
            .into_iter()
            .map(|strin| Section {
                page_address: None,
                r#type: "text".to_string(),
                content: format!("[{}](#)", strin),
                rendered_content: format!("<a href='#'>{}</a>", strin),
            })
            .collect();
        Ok(HomePage {
            title: "home page".to_string(),
            sections: vec,
        })
    }
    #[zome_fn("hc_public")]
    fn get_section(address: Address) -> ZomeApiResult<Section> {
        hdk::utils::get_as_type::<Section>(address)
    }

    #[zome_fn("hc_public")]
    fn update_element(address: Address, element: Section) -> ZomeApiResult<Address> {
        let old_element = hdk::utils::get_as_type::<Section>(address.clone())?;
        Section {
            page_address: old_element.page_address,
            r#type: element.r#type,
            content: element.content,
            rendered_content: element.rendered_content,
        }
        .update(address)
    }
}
