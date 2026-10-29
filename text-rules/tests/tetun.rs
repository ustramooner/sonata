//! Runs the Tetun rules file against the same cases as scripts/eval/test_tetun_text.py (the Python normaliser it replaces).
use sonata_text_rules::TextRules;

const CASES: &[(&str, &str)] = &[
    ("Reuniaun hahú tuku 17:00.", "Reuniaun hahú tuku 17."),
    ("Reuniaun hahú 17:00.", "Reuniaun hahú tuku 17."),
    ("tuku 10:30", "tuku 10 liu minutu 30"),
    ("Hahú 08:05", "Hahú tuku 8 liu minutu 5"),
    ("14h00 no 14h30", "tuku 14 no tuku 14 liu minutu 30"),
    ("Reuniaun iha loron 17/03/2022.", "Reuniaun iha loron 17, fulan Marsu, tinan 2022."),
    ("17-03-2022", "loron 17, fulan Marsu, tinan 2022"),
    ("Planu ba tinan 2022-2027.", "Planu ba tinan 2022-2027."),
    ("1,35", "1, virgula 35"),
    ("0,5", "0, virgula 5"),
    ("1,05", "1, virgula 0 5"),
    ("2,002", "2,002"),
    ("51.495 ema", "51.495 ema"),
    ("Orsamentu mak $1,35 biliaun.", "Orsamentu mak dolar biliaun 1, virgula 35."),
    ("5 milhaun", "miliaun 5"),
    ("10:30:15", "10:30:15"),
    ("Ema 11 mak mai.", "Ema 11 mak mai."),
    ("5mm", "milímetru 5"),
    ("Ninia naruk 5 cm.", "Ninia naruk sentímetru 5."),
    ("Todan 2,5 kg", "Todan kilograma 2, virgula 5"),
    ("10GB no 2.4GHz", "gigabait 10 no gigahertz 2.4"),
    ("Dr. Silva no Sra. Maria", "doutór Silva no senhora Maria"),
    ("Pr Joao hasoru Sr. Mario", "Pr Joao hasoru senhór Mario"),
    ("livru, kadeira sst.", "livru, kadeira ho selu-seluk tan"),
    ("no. 5", "númeru 5"),
    ("Ha'u ba 5 mak.", "Ha'u ba 5 mak."),
    ("km", "km"),
];

#[test]
fn tetun_cases() {
    let rules = TextRules::from_json(include_str!("../examples/tdt.textrules.json")).unwrap();
    for (text, want) in CASES {
        assert_eq!(&rules.apply(text), want, "input: {text}");
    }
}
