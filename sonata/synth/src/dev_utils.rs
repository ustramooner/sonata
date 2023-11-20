use sonata_synth::{AudioOutputConfig, AudioSamples, SonataModel, SonataResult, SonataSpeechSynthesizer};
use sonata_piper::from_config_path as voice_from_config_path;
use once_cell::sync::{OnceCell, Lazy};
use core::hint::black_box;
use std::path::PathBuf;
use std::sync::Arc;

const TEXT: &[&'static str] = &[
    "Technology is not inevitable.",
    "Powerful drivers must exist in order for people to keep pushing the envelope and continue demanding more and more from a particular field of knowledge.",
"Cheaper Communications",
"The first and most important driver is our demand for ever cheaper and easier communications.",
"All of human society depends on communications."
];
const CRATE_DIR: &'static str = env!("CARGO_MANIFEST_DIR");
static ORT_ENVIRONMENT: OnceCell<Arc<ort::Environment>> = OnceCell::new();
static STD_VOICE: Lazy<Arc<dyn SonataModel + Send + Sync>> = Lazy::new(|| {
    let config_path = model_directory("std").join("model.onnx.json");
    voice_from_config_path(&config_path, get_ort_environment()).unwrap()
});
static RT_VOICE: Lazy<Arc<dyn SonataModel + Send + Sync>> = Lazy::new(|| {
    let config_path = model_directory("rt").join("config.json");
    voice_from_config_path(&config_path, get_ort_environment()).unwrap()
});

#[allow(dead_code)]
pub fn init() {
    Lazy::force(&STD_VOICE);
    Lazy::force(&RT_VOICE);
}

fn get_ort_environment() -> &'static Arc<ort::Environment> {
    ORT_ENVIRONMENT.get_or_init(|| {
        Arc::new(
            ort::Environment::builder()
                .with_name("sonata")
                .with_execution_providers([ort::ExecutionProvider::CPU(Default::default())])
                .build()
                .unwrap(),
        )
    })
}

fn model_directory(kind: &str) -> PathBuf {
    PathBuf::from(CRATE_DIR).join("models").join(kind)
}

pub fn gen_params(kind: &str) -> (SonataSpeechSynthesizer, String, Option<AudioOutputConfig>) {
    let output_config = Some(AudioOutputConfig {
        rate: Some(50),
        volume: Some(50),
        pitch: Some(50),
        appended_silence_ms: None,
    });
    let text = TEXT.join("\n");
    if kind == "std" {
        let model = Arc::clone(&STD_VOICE);
        let synth = SonataSpeechSynthesizer::new(model).unwrap();
        (synth, text, output_config)
    } else if kind == "rt" {
        let model = Arc::clone(&RT_VOICE);
        let synth = SonataSpeechSynthesizer::new(model).unwrap();
        (synth, text, output_config)
    } else {
        panic!("Unknown parameterization  for function.")
    }
}

#[inline(always)]
pub fn iterate_stream(stream: impl Iterator<Item = SonataResult<AudioSamples>>)  -> SonataResult<()> {
    for result in stream {
        let audio = result?;
        let wav_bytes = black_box(audio.as_wave_bytes());
        wav_bytes.len();
    }
    Ok(())
}

