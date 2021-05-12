use crate::*;

#[derive(Debug)]
pub enum ParseError {
    UnrecognizedToken(String),
    UnexpectedToken(Token),
    UnexpectedEof,
}

pub type Result<T> = std::result::Result<T, ParseError>;

pub trait TokenStream: Iterator<Item = Token> {
    fn next_token(&mut self) -> Result<Token> {
        self.next().ok_or(ParseError::UnexpectedEof)
    }
}

impl<T: Iterator<Item = Token>> TokenStream for T {}

#[derive(Debug)]
pub enum Token {
    Bpm,
    Offset,
    TimeOffset(TimeOffset),
    Direction(Direction),
    Number(f32),
    Projectile(ProjectileType),
}

impl Token {
    pub fn parse(source: &str) -> Result<Self> {
        if let Ok(time_offset) = TimeOffset::parse(source) {
            return Ok(Self::TimeOffset(time_offset));
        }

        if let Ok(number) = source.parse::<f32>() {
            return Ok(Self::Number(number));
        }

        match source {
            "#bpm" => Ok(Self::Bpm),
            "#offset" => Ok(Self::Offset),
            "U" => Ok(Self::Direction(Direction::Up)),
            "D" => Ok(Self::Direction(Direction::Down)),
            "L" => Ok(Self::Direction(Direction::Left)),
            "R" => Ok(Self::Direction(Direction::Right)),
            "norm" => Ok(Self::Projectile(ProjectileType::Normal)),
            _ => Err(ParseError::UnrecognizedToken(source.to_string())),
        }
    }
}

pub fn parse_tokes(source: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();

    for s in source.split_whitespace() {
        tokens.push(Token::parse(s)?);
    }

    Ok(tokens)
}

#[derive(Debug)]
pub struct TimeOffset {
    pub fourths: u32,
    pub beats: u32,
    pub bars: u32,
}

impl TimeOffset {
    pub fn parse(mut source: &str) -> Result<Self> {
        let fourths = if let Some(index) = source.find(";") {
            let fourths = source[..index]
                .parse()
                .map_err(|_| ParseError::UnrecognizedToken(source[..index].to_string()))?;

            source = &source[index + 1..];

            fourths
        } else {
            0
        };

        if let Some(index) = source.find("|") {
            Ok(Self {
                fourths,
                beats: source[..index]
                    .parse()
                    .map_err(|_| ParseError::UnrecognizedToken(source[..index].to_string()))?,
                bars: source[index + 1..]
                    .parse()
                    .map_err(|_| ParseError::UnrecognizedToken(source[index + 1..].to_string()))?,
            })
        } else {
            Ok(Self {
                fourths,
                beats: source
                    .parse()
                    .map_err(|_| ParseError::UnrecognizedToken(source.to_string()))?,
                bars: 0,
            })
        }
    }

    pub fn time(&self, bpm: f32) -> f32 {
        let beat = 60.0 / bpm;

        self.fourths as f32 * beat / 4.0 + self.beats as f32 * beat + self.bars as f32 * beat * 4.0
    }
}

#[derive(Default)]
pub struct Sheet {
    pub bpm: f32,
    pub start_offset: f32,
    pub projectiles: Vec<Projectile>,
}

impl Sheet {
    pub fn parse(source: &str) -> Result<Self> {
        let mut sheet = Self::default();

        let mut tokens = parse_tokes(source)?.into_iter();

        sheet.parse_bpm(&mut tokens)?;
        sheet.parse_offset(&mut tokens)?;

        let mut start = sheet.start_offset;

        while tokens.len() > 0 {
            let projectile = Projectile::parse(&mut tokens, sheet.bpm, start)?;

            start = projectile.arrival_time;

            sheet.projectiles.push(projectile);
        }

        Ok(sheet)
    }

    pub fn parse_bpm(&mut self, tokens: &mut impl TokenStream) -> Result<()> {
        let bpm = tokens.next_token()?;

        if let Token::Bpm = bpm {
            let bpm = tokens.next_token()?;

            if let Token::Number(bpm) = bpm {
                self.bpm = bpm;

                Ok(())
            } else {
                Err(ParseError::UnexpectedToken(bpm))
            }
        } else {
            Err(ParseError::UnexpectedToken(bpm))
        }
    }

    pub fn parse_offset(&mut self, tokens: &mut impl TokenStream) -> Result<()> {
        let offset = tokens.next_token()?;

        if let Token::Offset = offset {
            let offset = tokens.next_token()?;

            if let Token::Number(offset) = offset {
                self.start_offset = offset;

                Ok(())
            } else {
                Err(ParseError::UnexpectedToken(offset))
            }
        } else {
            Err(ParseError::UnexpectedToken(offset))
        }
    }
}
