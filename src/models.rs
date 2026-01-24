use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Torrent {
    pub hash: String,
    pub name: String,
    #[serde(rename = "size")]
    pub size_bytes: i64,
    pub progress: f64,
    pub dlspeed: i64,
    pub upspeed: i64,
    pub priority: i64,
    #[serde(rename = "num_seeds")]
    pub num_seeds: i64,
    #[serde(rename = "num_leechs")]
    pub num_leechs: i64,
    #[serde(rename = "num_incomplete")]
    pub num_incomplete: i64,
    #[serde(rename = "num_complete")]
    pub num_complete: i64,
    pub ratio: f64,
    pub eta: i64,
    pub state: String,
    pub seq_dl: bool,
    pub f_l_piece_prio: bool,
    pub category: String,
    pub tags: String,
    pub super_seeding: bool,
    pub force_start: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TorrentFile {
    pub index: i64,
    pub name: String,
    pub size: i64,
    pub progress: f64,
    pub priority: i64,
    pub is_seed: Option<bool>,
    pub piece_range: Option<Vec<i64>>,
    pub availability: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TorrentProperties {
    pub save_path: String,
    pub creation_date: i64,
    pub piece_size: i64,
    pub comment: String,
    pub total_wasted: i64,
    pub total_uploaded: i64,
    pub total_downloaded: i64,
    pub up_limit: i64,
    pub dl_limit: i64,
    pub time_elapsed: i64,
    pub seeding_time: i64,
    pub nb_connections: i64,
    pub nb_connections_limit: i64,
    pub share_ratio: f64,
    pub addition_date: i64,
    pub completion_date: i64,
    pub created_by: String,
    pub dl_speed_avg: i64,
    pub dl_speed: i64,
    pub eta: i64,
    pub last_seen: i64,
    pub peers: i64,
    pub peers_total: i64,
    pub pieces_have: i64,
    pub pieces_num: i64,
    pub reannounce: i64,
    pub seeds: i64,
    pub seeds_total: i64,
    pub total_size: i64,
    pub up_speed_avg: i64,
    pub up_speed: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransferInfo {
    pub dl_info_speed: i64,
    pub dl_info_data: i64,
    pub up_info_speed: i64,
    pub up_info_data: i64,
    pub dl_rate_limit: i64,
    pub up_rate_limit: i64,
    pub dht_nodes: i64,
    pub connection_status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    #[serde(rename = "fileName")]
    pub file_name: String,
    #[serde(rename = "fileUrl")]
    pub file_url: String,
    #[serde(rename = "fileSize")]
    pub file_size: i64,
    #[serde(rename = "nbSeeders")]
    pub nb_seeders: i64,
    #[serde(rename = "nbLeechers")]
    pub nb_leechers: i64,
    #[serde(rename = "siteUrl")]
    pub site_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchJob {
    pub id: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchStatus {
    pub id: i64,
    pub status: String, // "Running", "Stopped"
    pub total: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResultsResponse {
    pub results: Vec<SearchResult>,
    pub status: String,
    pub total: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_torrent() {
        let json = r#"
        {
            "added_on": 1500000000,
            "amount_left": 0,
            "auto_tmm": false,
            "availability": 2,
            "category": "ISOs",
            "completed": 1000,
            "completion_on": 1500000500,
            "content_path": "/downloads/Ubuntu.iso",
            "dl_limit": -1,
            "dlspeed": 5000,
            "downloaded": 1000,
            "downloaded_session": 1000,
            "eta": 3600,
            "f_l_piece_prio": false,
            "force_start": false,
            "hash": "8c4a5c5b5d5e5f5g5h5i5j5k5l5m5n5o5p5q5r5s",
            "last_activity": 1500001000,
            "magnet_uri": "magnet:?xt=urn:btih:...",
            "max_ratio": -1,
            "max_seeding_time": -1,
            "name": "Ubuntu Linux",
            "num_complete": 10,
            "num_incomplete": 5,
            "num_leechs": 5,
            "num_seeds": 10,
            "priority": 1,
            "progress": 0.5,
            "ratio": 1.5,
            "ratio_limit": -2,
            "save_path": "/downloads/",
            "seeding_time": 600,
            "seeding_time_limit": -2,
            "seen_complete": 1500000500,
            "seq_dl": false,
            "size": 2000000000,
            "state": "downloading",
            "super_seeding": false,
            "tags": "linux,iso",
            "time_active": 1000,
            "total_size": 2000000000,
            "tracker": "http://tracker.example.com",
            "up_limit": -1,
            "uploaded": 500,
            "uploaded_session": 500,
            "upspeed": 1000
        }
        "#;

        let torrent: Torrent = serde_json::from_str(json).expect("Failed to deserialize torrent");

        assert_eq!(torrent.hash, "8c4a5c5b5d5e5f5g5h5i5j5k5l5m5n5o5p5q5r5s");
        assert_eq!(torrent.name, "Ubuntu Linux");
        assert_eq!(torrent.size_bytes, 2000000000);
        assert_eq!(torrent.state, "downloading");
        assert_eq!(torrent.progress, 0.5);
    }
}
