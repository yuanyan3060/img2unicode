use clap::Parser;
use image::imageops::FilterType;
use image::{DynamicImage, GrayImage, ImageResult, Luma};
use imageproc::contrast::adaptive_threshold;

use std::path::Path;
use std::{char, iter, u32};

fn next_multiple_ge_x(x: u32, y: u32) -> u32 {
    if x % y == 0 {
        x
    } else {
        (x / y + 1) * y
    }
}
pub fn open_img(path: impl AsRef<Path>, w: Option<u32>) -> ImageResult<GrayImage> {
    let img = image::open(path)?.to_luma8();
    match w {
        Some(w) => {
            let img = DynamicImage::from(img);
            let img = img.resize(w, u32::MAX, FilterType::Lanczos3);
            Ok(img.to_luma8())
        }
        None => Ok(img),
    }
}

#[derive(Clone, Copy)]
pub enum State {
    S0 = 0,
    S1 = 1,
    S2 = 2,
    S3 = 3,
}

impl State {
    pub fn next_state(&self) -> Self {
        match self {
            State::S0 => State::S1,
            State::S1 => State::S2,
            State::S2 => State::S3,
            State::S3 => State::S0,
        }
    }
}

pub enum ThresholdMethod {
    Fixed(u8),
    Adaptive(u32),
}

pub fn to_unicode(src: &GrayImage, method: ThresholdMethod) -> String {
    let bind;
    let gray;
    let threshold;

    match method {
        ThresholdMethod::Fixed(val) => {
            threshold = val;
            gray = src;
        }
        ThresholdMethod::Adaptive(val) => {
            threshold = 150;
            bind = adaptive_threshold(src, val);
            gray = &bind;
        }
    }

    let w = next_multiple_ge_x(gray.width(), 2);
    let pad_w = w - gray.width();
    let h = next_multiple_ge_x(gray.height(), 4);

    let mut buf = vec![10240u32; ((w as usize / 2) + 1) * (h as usize / 4)];
    let mut state = State::S0;

    for (y, row) in gray.rows().enumerate() {
        let row = row.chain(iter::repeat_n(&Luma::<u8>([0]), pad_w as usize));
        let start = (y / 4) * (w as usize / 2);
        let end = start + (w as usize / 2);
        let sub_buf = &mut buf[start..end];
        *sub_buf.last_mut().unwrap() = '\n' as u32;
        for (x, p) in row.into_iter().copied().enumerate() {
            if p.0[0] <= threshold {
                continue;
            }
            let v = match (state, x % 2 == 0) {
                (State::S0, true) => 1 << 3,
                (State::S0, false) => 1 << 0,
                (State::S1, true) => 1 << 4,
                (State::S1, false) => 1 << 1,
                (State::S2, true) => 1 << 5,
                (State::S2, false) => 1 << 2,
                (State::S3, true) => 1 << 7,
                (State::S3, false) => 1 << 6,
            };
            sub_buf[x / 2] += v;
        }
        *sub_buf.last_mut().unwrap() = '\n' as u32;
        state = state.next_state();
    }

    unsafe {
        buf.into_iter()
            .map(|x| char::from_u32_unchecked(x))
            .collect()
    }
}

#[derive(Parser, Debug)]
pub struct Args {
    path: String,

    #[arg(short, long)]
    width: Option<u32>,

    #[arg(short, long, conflicts_with = "block_radius")]
    threshold: Option<u8>,

    #[arg(short, long, conflicts_with = "threshold")]
    block_radius: Option<u32>,
}

fn main() -> ImageResult<()> {
    let args = Args::parse();
    let img = open_img(&args.path, args.width)?;
    let mut method = ThresholdMethod::Fixed(127);
    if let Some(threshold) = args.threshold {
        method = ThresholdMethod::Fixed(threshold)
    }

    if let Some(block_radius) = args.block_radius {
        method = ThresholdMethod::Adaptive(block_radius)
    }
    println!("{}", to_unicode(&img, method));
    Ok(())
}
