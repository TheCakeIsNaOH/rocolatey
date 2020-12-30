use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};

use crate::roco::local::get_local_packages;
use crate::roco::{get_choco_sources, semver_is_newer, Feed, OutdatedInfo, Package};

// https://rust-lang-nursery.github.io/rust-cookbook/web/clients/download.html
// https://joelverhagen.github.io/NuGetUndocs/
// http://docs.oasis-open.org/odata/odata/v4.0/errata03/os/complete/part1-protocol/odata-v4.0-errata03-os-part1-protocol-complete.html

async fn get_package_count_on_feed(f: &Feed, prerelease: bool) -> u32 {
  let latest_filter = match prerelease {
    true => "$filter=IsAbsoluteLatestVersion",
    false => "$filter=IsLatestVersion",
  };
  let rs = format!("{}/Packages()/$count?{}", f.url, latest_filter);
  let resp_pkg_count = reqwest::get(&rs).await;
  let total_pkg_count = resp_pkg_count.unwrap().text().await.unwrap();
  let total_pkg_count = total_pkg_count.parse::<u32>().unwrap();
  total_pkg_count
}

async fn receive_package_delta(
  feed: &Feed,
  batch_size: u32,
  batch_offset: u32,
  prerelease: bool,
) -> (u32, String) {
  let base_uri = format!("{}/Packages()", feed.url);
  let latest_filter = match prerelease {
    true => "$filter=IsAbsoluteLatestVersion",
    false => "$filter=IsLatestVersion",
  };
  let rs = match batch_size {
    0 => format!("{}?{}&$skip={}", base_uri, latest_filter, batch_offset),
    _ => format!(
      "{}?{}&$top={}&$skip={}",
      base_uri, latest_filter, batch_size, batch_offset
    ),
  };
  // println!("q: {}", rs);
  let resp = reqwest::get(&rs).await;
  let query_res = resp.unwrap().text().await.unwrap();
  let c = query_res.matches("</entry>").count();
  (c as u32, query_res)
}

async fn update_feed_index(feed: &Feed, limitoutput: bool, prerelease: bool) -> String {
  let total_pkg_count = get_package_count_on_feed(feed, prerelease).await;
  println!(
    "there are a total of {} packages on feed {}",
    total_pkg_count, feed.name
  );
  let f = File::create(format!("{}_dl.xml", feed.name)).expect("Unable to create file");
  let mut f = BufWriter::new(f);
  let mut batch_size = 0;
  let mut received_packages = 0;
  let progress_bar = match limitoutput {
    true => indicatif::ProgressBar::hidden(),
    false => indicatif::ProgressBar::new(total_pkg_count as u64),
  };
  while received_packages < total_pkg_count {
    let (a, req_res) = receive_package_delta(feed, batch_size, received_packages, prerelease).await;
    if a != batch_size {
      println!("receiving packages in batches of {} per request", a);
    }
    batch_size = a;
    f.write_all(req_res.as_bytes())
      .expect("unable to write data");

    received_packages += batch_size;
    progress_bar.set_position(received_packages as u64);
  }
  progress_bar.finish();
  f.flush().expect("failed to flush file buffer");
  // TODO - "shrink" pkg index files - only keep id + version (faster lookup later on)

  String::from("update_feed_index -> not implemented")
}

pub async fn update_package_index(limitoutput: bool, prerelease: bool) -> String {
  let mut s = String::new();
  let feeds = get_choco_sources().expect("failed to get choco sources");
  for f in feeds {
    s.push_str(&update_feed_index(&f, limitoutput, prerelease).await);
  }
  s
}

async fn get_latest_remote_packages_on_feed(
  pkgs: &Vec<Package>,
  feed: &Feed,
  prerelease: bool,
) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
  // else - recurse file search + filename analysis
  let https_regex = regex::Regex::new(r"^https?://.+").unwrap();
  match https_regex.is_match(&feed.url) {
    true => {
      let odata_xml = get_odata_xml_packages(pkgs, feed, prerelease)
        .await
        .expect("failed to receive odata for packages");
      Ok(get_packages_from_odata(&odata_xml))
    }
    false => {
      let nupkg_files = get_nupkgs_from_path(pkgs, feed, prerelease)
        .expect("failed to read package info from file system");
      Ok(get_packages_from_nupkg(&nupkg_files))
    }
  }
}

fn get_nupkgs_from_path(
  pkgs: &Vec<Package>,
  feed: &Feed,
  prerelease: bool,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
  println!("! WARNING ! - get_nupkgs_from_path NOT YET IMPLEMENTED!");
  println!("! WARNING ! - only http / https - are evaluated\n");
  // TODO implement
  Ok(Vec::new())
}

fn get_packages_from_nupkg(nupkg_files: &Vec<String>) -> Vec<Package> {
  // TODO implement
  Vec::new()
}

async fn get_latest_remote_packages(
  pkgs: &Vec<Package>,
  feeds: &Vec<Feed>,
  prerelease: bool,
) -> Result<HashMap<String, Package>, Box<dyn std::error::Error>> {
  let mut remote_pkgs: HashMap<String, Package> = HashMap::new();

  for f in feeds {
    let pkgs = get_latest_remote_packages_on_feed(pkgs, f, prerelease)
      .await
      .expect("failed to get remote packages");
    // println!("{:#?}", pkgs);
    for p in pkgs {
      if remote_pkgs.contains_key(&p.id) {
        if !semver_is_newer(&remote_pkgs.get(&p.id).unwrap().version, &p.version) {
          continue;
        }
      }
      remote_pkgs.insert(p.id.to_lowercase(), p);
    }
  }

  Ok(remote_pkgs)
}

pub async fn get_outdated_packages(limitoutput: bool, prerelease: bool) -> String {
  // foreach local package, compare remote version number
  let local_packages = get_local_packages().expect("failed to get local package list");
  let remote_feeds = get_choco_sources().expect("failed to get choco feeds");
  let latest_packages = get_latest_remote_packages(&local_packages, &remote_feeds, prerelease)
    .await
    .expect("failed to get remote package list");

  let mut oi: Vec<OutdatedInfo> = Vec::new();
  let mut warning_count = 0;
  for l in local_packages {
    match latest_packages.get(&l.id.to_lowercase()) {
      Some(u) => {
        if semver_is_newer(&u.version, &l.version) {
          oi.push(OutdatedInfo {
            id: l.id,
            local_version: l.version.clone(),
            remote_version: u.version.clone(),
            pinned: l.pinned,
            exists_on_remote: true,
          });
        }
      }
      None => {
        warning_count += 1;
        oi.push(OutdatedInfo {
          id: l.id,
          local_version: l.version.clone(),
          remote_version: l.version.clone(),
          pinned: l.pinned,
          exists_on_remote: false,
        })
      }
    };
  }
  let mut warnings = String::new();
  let mut res = String::new();

  if !limitoutput {
    res.push_str("Outdated Packages\n");
    res.push_str(" Output is package name | current version | available version | pinned?\n\n");
  }

  let mut outdated_packages = 0;
  for o in oi {
    outdated_packages += 1;
    res.push_str(&format!(
      "{}|{}|{}|{}\n",
      o.id, o.local_version, o.remote_version, o.pinned
    ));
    if !o.exists_on_remote {
      warnings.push_str(&format!(" - {}\n", o.id));
    }
  }

  if !limitoutput {
    res.push_str(&format!(
      "\nRocolatey has determined {} package(s) are outdated.\n",
      outdated_packages
    ));
    res.push_str(&format!(" {} packages(s) had warnings.\n", warning_count));
    res.push_str(&format!("Warnings:\n"));
    res.push_str(&warnings);
  }
  res
}

async fn get_odata_xml_packages(
  pkgs: &Vec<Package>,
  feed: &Feed,
  prerelease: bool,
) -> Result<String, Box<dyn std::error::Error>> {
  let mut query_res = String::new();
  let latest_filter = match prerelease {
    true => "IsAbsoluteLatestVersion",
    false => "IsLatestVersion",
  };
  let query_string_base: String = format!("{}/Packages?$filter={}", feed.url, latest_filter);
  let total_pkgs = pkgs.len();
  let mut received_pkgs = 0;
  let mut curr_pkg_idx = 0;

  // https://chocolatey.org/api/v2/Packages?$filter=IsLatestVersion and (Id eq 'Chocolatey' or Id eq 'Boxstarter' or Id eq 'vscode' or Id eq 'notepadplusplus')

  // NOTE: some feeds may have pagination (such as choco community repo)
  // need to impl some way to determine maximum possible batch_size!
  let max_batch_size = 39;

  while received_pkgs < total_pkgs {
    let mut query_string = format!("{} and (", query_string_base);
    let mut batch_size = 0;
    loop {
      query_string.push_str(&format!("Id eq '{}'", pkgs.get(curr_pkg_idx).unwrap().id));
      curr_pkg_idx += 1;
      batch_size += 1;

      if (query_string.len() > 2000) || curr_pkg_idx == pkgs.len() || batch_size >= max_batch_size {
        query_string.push_str(")");
        break;
      }
      query_string.push_str(" or ");
    }

    // println!(" -> q: {}", query_string);

    let resp_odata = reqwest::get(&query_string).await;
    let resp_odata = resp_odata.unwrap().text().await.unwrap();
    query_res.push_str(&resp_odata);
    // note: not all queried pkgs have to exist on remote, thus we always need to inc batch_size,
    // no matter if the queried pkgs were received or not!
    received_pkgs += batch_size;
  }

  Ok(query_res)
}

fn get_packages_from_odata(odata_xml: &str) -> Vec<Package> {
  let mut packages = Vec::new();
  let mut pkg_name = String::new();
  let mut pkg_version = String::new();

  let mut reader = Reader::from_str(odata_xml);
  reader.trim_text(true);
  let mut buf = Vec::new();

  // entry/title -> id
  // entry/m:properties/d:Version -> Version

  enum ODataParserState {
    LookingForEntry,
    InEntry,
    InEntryId,
    InEntryVersion,
  }

  let mut state = ODataParserState::LookingForEntry;

  loop {
    match reader.read_event(&mut buf) {
      Ok(Event::Start(ref e)) => match e.name() {
        b"entry" => state = ODataParserState::InEntry,
        b"title" => match state {
          ODataParserState::InEntry => {
            state = ODataParserState::InEntryId;
          }
          _ => {}
        },
        b"d:Version" => match state {
          ODataParserState::InEntry => {
            state = ODataParserState::InEntryVersion;
          }
          _ => {}
        },
        _ => {}
      },
      Ok(Event::Text(e)) => match state {
        ODataParserState::InEntryId => {
          pkg_name = String::from_utf8(e.to_vec()).unwrap();
        }
        ODataParserState::InEntryVersion => pkg_version = String::from_utf8(e.to_vec()).unwrap(),
        _ => (),
      },
      Ok(Event::End(ref e)) => match e.name() {
        b"entry" => {
          packages.push(Package {
            id: pkg_name.to_string(),
            version: pkg_version.to_string(),
            pinned: false,
          });
          state = ODataParserState::LookingForEntry;
        }
        b"title" => match state {
          ODataParserState::InEntryId => {
            state = ODataParserState::InEntry;
          }
          _ => {}
        },
        b"d:Version" => match state {
          ODataParserState::InEntryVersion => {
            state = ODataParserState::InEntry;
          }
          _ => {}
        },
        _ => {}
      },
      Ok(Event::Eof) => break,
      _ => (),
    }
    buf.clear();
  }

  packages
}