/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use calcard::{
    common::timezone::Tz, icalendar::dates::CalendarExpand, jscalendar::JSCalendar,
    jscontact::JSContact, Entry, Parser,
};
use leptos::*;
use leptos_meta::*;
use rand::seq::SliceRandom;
use std::borrow::Cow;

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    leptos::mount_to_body(|| view! { <App/> })
}

#[derive(Clone, Debug)]
struct Occurrence {
    from: String,
    to: String,
}

#[derive(Clone, Copy, Debug)]
enum SourceType {
    ICalendar,
    JSCalendar,
    VCard,
    JSContact,
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let source = create_rw_signal(String::new());
    let source_type = create_rw_signal(SourceType::ICalendar);
    let conversion = create_rw_signal(String::new());
    let roundtrip_conversion = create_rw_signal(String::new());
    let error_message = create_rw_signal(String::new());
    let occurrences: RwSignal<Vec<Occurrence>> = create_rw_signal(vec![]);

    let set_error = move |msg: String| {
        error_message.set(msg);
        conversion.set(String::new());
        roundtrip_conversion.set(String::new());
        occurrences.set(vec![]);
    };

    let set_occurrences = move |expanded: CalendarExpand| {
        let mut events = expanded
            .events
            .into_iter()
            .filter_map(|event| event.try_into_date_time())
            .collect::<Vec<_>>();
        events.sort_unstable_by(|a, b| a.start.cmp(&b.start));
        occurrences.set(
            events
                .into_iter()
                .map(|event| Occurrence {
                    from: format!(
                        "{} ({})",
                        event.start.format("%a %b %-d, %Y %-I:%M%P"),
                        event
                            .start
                            .timezone()
                            .name()
                            .unwrap_or(Cow::Borrowed("Floating"))
                    ),
                    to: format!(
                        "{} ({})",
                        event.end.format("%a %b %-d, %Y %-I:%M%P"),
                        event
                            .end
                            .timezone()
                            .name()
                            .unwrap_or(Cow::Borrowed("Floating"))
                    ),
                })
                .collect(),
        );
    };

    let convert = move || {
        let source = source.get();
        let source = source.trim_start();
        occurrences.set(vec![]);
        error_message.set(String::new());

        if source.is_empty() {
            return;
        }

        if source.starts_with("BEGIN:") {
            match Parser::new(source).entry() {
                Entry::VCard(vcard) => {
                    source_type.set(SourceType::VCard);
                    let jscontact = vcard.into_jscontact();
                    conversion.set(jscontact.to_string_pretty());
                    match jscontact.into_vcard() {
                        Some(vcard_roundtrip) => {
                            roundtrip_conversion.set(vcard_roundtrip.to_string());
                        }
                        None => {
                            set_error("Looks like you've found a bug in the conversion. Please report it.".to_string());
                        }
                    }
                }
                Entry::ICalendar(icalendar) => {
                    source_type.set(SourceType::ICalendar);
                    set_occurrences(icalendar.expand_dates(Tz::Floating, 25));
                    let jscalendar = icalendar.into_jscalendar();
                    conversion.set(jscalendar.to_string_pretty());
                    match jscalendar.into_icalendar() {
                        Some(icalendar_roundtrip) => {
                            roundtrip_conversion.set(icalendar_roundtrip.to_string());
                        }
                        None => {
                            set_error("Looks like you've found a bug in the conversion. Please report it.".to_string());
                        }
                    }
                }
                Entry::InvalidLine(text) => {
                    set_error(format!("Invalid line found: {}", text));
                }
                Entry::UnexpectedComponentEnd { expected, found } => {
                    set_error(format!(
                        "Unexpected component end: expected {}, found {}",
                        expected.as_str(),
                        found.as_str()
                    ));
                }
                Entry::UnterminatedComponent(cow) => {
                    set_error(format!("Unterminated component: {}", cow));
                }
                Entry::TooManyComponents => {
                    set_error("Too many components".to_string());
                }
                Entry::Eof => {
                    set_error("Unexpected end of file".to_string());
                }
                _ => todo!(),
            }
        } else if source.starts_with('{') {
            if source.contains("\"Group\"") {
                match JSCalendar::parse(source.trim_end()) {
                    Ok(jscalendar) => match jscalendar.into_icalendar() {
                        Some(icalendar) => {
                            source_type.set(SourceType::JSCalendar);
                            conversion.set(icalendar.to_string());
                            set_occurrences(icalendar.expand_dates(Tz::Floating, 25));
                            roundtrip_conversion
                                .set(icalendar.into_jscalendar().to_string_pretty());
                        }
                        None => {
                            set_error("Looks like you've found a bug in the conversion. Please report it.".to_string());
                        }
                    },
                    Err(err) => {
                        set_error(format!("Failed to parse JSCalendar: {}", err));
                    }
                }
            } else if source.contains("\"Card\"") {
                match JSContact::parse(source) {
                    Ok(jscontact) => match jscontact.into_vcard() {
                        Some(vcard) => {
                            source_type.set(SourceType::JSContact);
                            conversion.set(vcard.to_string());
                            roundtrip_conversion.set(vcard.into_jscontact().to_string_pretty());
                        }
                        None => {
                            set_error("Looks like you've found a bug in the conversion. Please report it.".to_string());
                        }
                    },
                    Err(err) => {
                        set_error(format!("Failed to parse JSContact: {}", err));
                    }
                }
            } else {
                set_error("This does not look like a valid JSCalendar or JSContact.".to_string());
            }
        } else {
            set_error("Unrecognized format. Please provide a valid iCalendar, JSCalendar, vCard or JSContact file.".to_string());
        }
    };

    view! {
        <Body class="dark:bg-slate-900 bg-gray-100 "/>

        <div class="max-w-4xl px-4 py-10 sm:px-6 lg:px-8 mx-auto">
            <div class="bg-white rounded-xl shadow-xs p-4 sm:p-7 dark:bg-neutral-800">
                <div class="mb-8">
                    <h2 class="text-xl font-bold text-gray-800 dark:text-neutral-200">
                        JSCalendar and JSContact conversion
                    </h2>
                    <p class="text-sm text-gray-600 dark:text-neutral-400">
                        "Bi-directional conversion from/to JSCalendar/iCalendar and JSContact/vCard."
                    </p>
                </div>

                <Show when=move || !error_message.get().is_empty()>
                    <div class="mb-6">
                        <div class="bg-red-50 border border-red-200 text-sm text-red-800 rounded-lg p-4 dark:bg-red-800/10 dark:border-red-900 dark:text-red-500">
                            <div class="flex">
                                <div class="shrink-0">
                                    <svg
                                        class="shrink-0 size-4 mt-0.5"
                                        xmlns="http://www.w3.org/2000/svg"
                                        width="24"
                                        height="24"
                                        viewBox="0 0 24 24"
                                        fill="none"
                                        stroke="currentColor"
                                        stroke-width="2"
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                    >
                                        <circle cx="12" cy="12" r="10"></circle>
                                        <path d="m15 9-6 6"></path>
                                        <path d="m9 9 6 6"></path>
                                    </svg>
                                </div>
                                <div class="ms-4">
                                    <h3 id="hs-with-list-label" class="text-sm font-semibold">
                                        {move || error_message.get()}
                                    </h3>
                                </div>
                            </div>
                        </div>
                    </div>
                </Show>

                <div class="relative">
                    <textarea
                        class="p-3 sm:p-4 pb-12 sm:pb-12 block w-full bg-gray-100 border-gray-200 rounded-lg sm:text-sm focus:border-blue-500 focus:ring-blue-500 dark:bg-neutral-800 dark:border-neutral-700 dark:text-neutral-400 dark:placeholder-neutral-500 dark:focus:ring-neutral-600"
                        autocapitalize="off"
                        rows="10"
                        placeholder="Paste here an iCalendar, JSCalendar, vCard or JSContact file. Or click the sparkles to try a sample."
                        prop:value=move || source.get()
                        on:change=move |ev| {
                            source
                                .update(|data| {
                                    *data = event_target_value(&ev);
                                });
                            convert();
                        }
                    >
                    </textarea>

                    <div class="absolute bottom-px inset-x-px p-2 rounded-b-lg bg-gray-100 dark:bg-neutral-800">
                        <div class="flex flex-wrap justify-between items-center gap-2">
                            <div class="flex items-center">
                                <p class="text-xs text-gray-500 dark:text-neutral-500"></p>
                            </div>
                            <div class="flex items-center gap-x-1">
                                <button
                                    type="button"
                                    class="inline-flex shrink-0 justify-center items-center size-8 rounded-lg text-gray-500 hover:bg-white focus:z-10 focus:outline-hidden focus:bg-white dark:text-neutral-500 dark:hover:bg-neutral-700 dark:focus:bg-neutral-700"
                                    on:click=move |_| {
                                        source
                                            .set(
                                                SAMPLES
                                                    .choose(&mut rand::thread_rng())
                                                    .unwrap_or(&"")
                                                    .to_string(),
                                            );
                                        convert();
                                    }
                                >

                                    <svg
                                        class="shrink-0 size-4"
                                        xmlns="http://www.w3.org/2000/svg"
                                        width="24"
                                        height="24"
                                        viewBox="0 0 24 24"
                                        fill="none"
                                        stroke="currentColor"
                                        stroke-width="2"
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                    >
                                        <path d="M9.813 15.904 9 18.75l-.813-2.846a4.5 4.5 0 0 0-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 0 0 3.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 0 0 3.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 0 0-3.09 3.09ZM18.259 8.715 18 9.75l-.259-1.035a3.375 3.375 0 0 0-2.455-2.456L14.25 6l1.036-.259a3.375 3.375 0 0 0 2.455-2.456L18 2.25l.259 1.035a3.375 3.375 0 0 0 2.456 2.456L21.75 6l-1.035.259a3.375 3.375 0 0 0-2.456 2.456ZM16.894 20.567 16.5 21.75l-.394-1.183a2.25 2.25 0 0 0-1.423-1.423L13.5 18.75l1.183-.394a2.25 2.25 0 0 0 1.423-1.423l.394-1.183.394 1.183a2.25 2.25 0 0 0 1.423 1.423l1.183.394-1.183.394a2.25 2.25 0 0 0-1.423 1.423Z"></path>
                                    </svg>
                                </button>
                                <button
                                    type="button"
                                    class="inline-flex shrink-0 justify-center items-center size-8 rounded-lg text-white bg-blue-600 hover:bg-blue-500 focus:z-10 focus:outline-hidden focus:bg-blue-500"
                                    on:click=move |_| {
                                        convert();
                                    }
                                >

                                    <svg
                                        class="shrink-0 size-3.5"
                                        xmlns="http://www.w3.org/2000/svg"
                                        width="16"
                                        height="16"
                                        fill="currentColor"
                                        viewBox="0 0 16 16"
                                    >
                                        <path d="M15.964.686a.5.5 0 0 0-.65-.65L.767 5.855H.766l-.452.18a.5.5 0 0 0-.082.887l.41.26.001.002 4.995 3.178 3.178 4.995.002.002.26.41a.5.5 0 0 0 .886-.083l6-15Zm-1.833 1.89L6.637 10.07l-.215-.338a.5.5 0 0 0-.154-.154l-.338-.215 7.494-7.494 1.178-.471-.47 1.178Z"></path>
                                    </svg>

                                </button>
                            </div>
                        </div>
                    </div>
                </div>

            </div>
        </div>

        <Show when=move || !conversion.get().is_empty()>
            <div class="max-w-4xl px-4 sm:px-6 lg:px-8 mx-auto pb-10">
                <div class="bg-white rounded-xl shadow-xs p-4 sm:p-7 dark:bg-neutral-800">
                    <div class="mb-4">
                        <h2 class="text-xl font-bold text-gray-800 dark:text-neutral-200">
                            Conversion results
                        </h2>

                    </div>
                    <p class="text-sm text-gray-600 dark:text-neutral-400 mb-4">
                        {format!(
                            "This is how your {} looks like in {} format:",
                            source_type.get().as_str(),
                            source_type.get().counterpart().as_str(),
                        )}

                    </p>
                    <div class="bg-gray-100 dark:bg-neutral-700 rounded-lg p-4 overflow-x-auto">
                        <pre class="text-sm text-gray-800 dark:text-neutral-200 whitespace-pre">
                            {move || conversion.get()}
                        </pre>
                    </div>
                    <p class="text-sm text-gray-600 dark:text-neutral-400 mt-4 mb-4">
                        {format!(
                            "And this is how it would look like converted back to {}:",
                            source_type.get().as_str(),
                        )}

                    </p>
                    <div class="bg-gray-100 dark:bg-neutral-700 rounded-lg p-4 overflow-x-auto">
                        <pre class="text-sm text-gray-800 dark:text-neutral-200 whitespace-pre">
                            {move || roundtrip_conversion.get()}
                        </pre>
                    </div>
                    <div class="flex justify-end gap-4 mt-3">
                        <p class="text-xs text-gray-600">
                            {format!("v{}", env!("CARGO_PKG_VERSION"))}
                        </p>
                        <a
                            href="https://github.com/stalwartlabs/calcard/issues/new"
                            class="text-xs text-blue-600 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300 hover:underline"
                            target="_blank"
                        >
                            Report a bug
                        </a>
                        <a
                            href="#"
                            class="text-xs text-blue-600 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300 hover:underline"
                            href="https://github.com/stalwartlabs/jmap-convert/tree/main"
                            target="_blank"
                        >
                            View source
                        </a>
                    </div>

                </div>
            </div>
        </Show>

        <Show when=move || !occurrences.get().is_empty()>
            <div class="max-w-4xl px-4 sm:px-6 lg:px-8 mx-auto">
                <div class="bg-white rounded-xl shadow-xs p-4 sm:p-7 dark:bg-neutral-800">
                    <div class="mb-4">
                        <h2 class="text-xl font-bold text-gray-800 dark:text-neutral-200">
                            Calendar expansion results
                        </h2>

                    </div>
                    <p class="text-sm text-gray-600 dark:text-neutral-400 mb-4">
                        {format!(
                            "These are the first {} occurrences of the pasted calendar event:",
                            occurrences.get().len(),
                        )}

                    </p>

                    <div class="flex flex-col">
                        <div class="-m-1.5 overflow-x-auto">
                            <div class="p-1.5 min-w-full inline-block align-middle">
                                <div class="border border-gray-200 overflow-hidden dark:border-neutral-700">
                                    <table class="min-w-full divide-y divide-gray-200 dark:divide-neutral-700">
                                        <thead>
                                            <tr>
                                                <th
                                                    scope="col"
                                                    class="px-6 py-3 text-start text-xs font-medium text-gray-500 uppercase dark:text-neutral-500"
                                                >
                                                    From date
                                                </th>
                                                <th
                                                    scope="col"
                                                    class="px-6 py-3 text-start text-xs font-medium text-gray-500 uppercase dark:text-neutral-500"
                                                >
                                                    To date
                                                </th>
                                            </tr>
                                        </thead>
                                        <tbody class="divide-y divide-gray-200 dark:divide-neutral-700">
                                            <For
                                                each=move || occurrences.get()
                                                key=move |occurrence| occurrence.from.clone()
                                                children=move |occurrence| {
                                                    view! {
                                                        <tr>
                                                            <td class="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-800 dark:text-neutral-200">
                                                                {occurrence.from}
                                                            </td>
                                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-800 dark:text-neutral-200">
                                                                {occurrence.to}
                                                            </td>
                                                        </tr>
                                                    }
                                                }
                                            />

                                        </tbody>
                                    </table>
                                </div>
                            </div>
                        </div>
                    </div>

                </div>
            </div>
        </Show>
    }
}

const SAMPLES: &[&str] = &[
    include_str!("../resources/ical_001.ics"),
    include_str!("../resources/ical_002.ics"),
    include_str!("../resources/ical_003.ics"),
    include_str!("../resources/vcard_001.vcf"),
    include_str!("../resources/vcard_002.vcf"),
    include_str!("../resources/vcard_003.vcf"),
    include_str!("../resources/jscal_001.json"),
    include_str!("../resources/jscal_002.json"),
    include_str!("../resources/jscal_003.json"),
    include_str!("../resources/jscontact_001.json"),
    include_str!("../resources/jscontact_002.json"),
];

impl SourceType {
    fn as_str(&self) -> &str {
        match self {
            SourceType::ICalendar => "iCalendar",
            SourceType::JSCalendar => "JSCalendar",
            SourceType::VCard => "vCard",
            SourceType::JSContact => "JSContact",
        }
    }

    fn counterpart(&self) -> SourceType {
        match self {
            SourceType::ICalendar => SourceType::JSCalendar,
            SourceType::JSCalendar => SourceType::ICalendar,
            SourceType::VCard => SourceType::JSContact,
            SourceType::JSContact => SourceType::VCard,
        }
    }
}
