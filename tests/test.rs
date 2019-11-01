use hancock_read_bin::{HancockReader, HancockDataRow};

#[test]
fn test_divide() {
    let test = HancockDataRow {
        zen: 0.0,
        az: 0.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
        shot_n: 0,
        n_hits: 0,
        r: vec![0.0],
        refl: vec![0.0],
    };
    assert_eq!(test.shot_n, 0);
}

#[test]
fn reader() {
    let mut reader = HancockReader::new(String::from("R:/bin_clouds/Plot_113_P113_day1_loc1_001_180731_113402.bin")).unwrap();
    if let Some(val) = reader.next() {
        assert_eq!(val.zen, 76.957695);
    }
}