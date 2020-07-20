use anyhow::anyhow;
use anyhow::Result;
use mdbook::book::Book;
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use mdbook::BookItem;
use pulldown_cmark::{Event, Options, Parser};
use pulldown_cmark_to_cmark::cmark;

/// A no-op preprocessor.

pub struct FixCJKSpacing;

impl FixCJKSpacing {
    pub fn new() -> Self {
        Self
    }
}

impl Preprocessor for FixCJKSpacing {
    fn name(&self) -> &str {
        "fix-cjk-spacing"
    }

    fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> Result<Book> {
        book.for_each_mut(|section: &mut BookItem| {
            if let BookItem::Chapter(ref mut ch) = *section {
                if let Ok(content) = join_cjk_spacing(&ch.content) {
                    ch.content = content;
                }
            }
        });
        Ok(book)
    }

    fn supports_renderer(&self, _renderer: &str) -> bool {
        true
    }
}

// - http://unicode-table.com/cn/
// - https://en.wikipedia.org/wiki/Unicode_block
//
// * 2000-206F General Punctuation
// * 2E80-2EFF 中日韩汉字部首补充
// * 2F00-2FDF 康熙部首
// * 3000-303F 中日韩符号和标点
// * 3040-309F 日文平假名 (V)
// * 30A0-30FF 日文片假名 (V)
// * 3100-312F 注音字母 (V)
// * 3130-318F 谚文相容字母（韩）
// * 3190-319F 汉文训读
// * 31A0-31BF 注音符号扩展
// * 31C0-31EF 中日韩笔画
// * 31F0-31FF 片假名拼音扩展
// * 3200-32FF 带圈中日韩字母和月份 (V)
// * 3300-33FF 中日韩兼容
// * 3400-4DBF 中日韩统一表意文字扩展区A
// * 4DC0-4DFF 易经六十四卦符号
// * 4E00-9FFF 中日韩统一表意文字
// * AC00-D7AF 谚文音节（韩）
// * D7B0-D7FF 谚文字母扩展-B（韩）
// * F900-FAFF 中日韩相容表意文字
// * FE30-FE4F 中日韩相容形式
// * FE50-FE6F 小写变体形式
// * FF00-FFEE Halfwidth and Fullwidth Forms
//
// Below are not processed
// * 16FE0-16FFF 表意符号和标点符号
// * 1B000-1B0FF 日文假名补充
// * 1B100-1B12F 日文假名扩展-A
// * 1D300-1D35F 太玄经符号
// * 1D360-1D37F 算筹
// * 1F000-1F02F 麻将牌
// * 1F100-1F1FF 带圈字母数字补充
// * 1F200-1F2FF 带圈表意文字补充
// * 1F300-1F5FF 杂项符号和象形文字
// * 20000-2A6DF 中日韩统一表意文字扩展区B
// * 2A700-2B73F 中日韩统一表意文字扩展区C
// * 2B740-2B81F 中日韩统一表意文字扩展区D
// * 2B820-2CEAF 中日韩统一表意文字扩展区E
// * 2CEB0-2EBEF 中日韩统一表意文字扩展区F
// * 2F800-2FA1F 中日韩相容表意文字补充区
// * 30000-3134F 中日韩统一表意文字扩展区G
fn is_cjk(ch: char) -> bool {
    match ch {
        '\u{2000}'..='\u{206F}' => true, // General Punctuation
        '\u{2E80}'..='\u{2EDF}' => true, // 中日韩汉字部首补充 | 康熙部首
        // 中日韩符号和标点 | 日文平假名 | 日文片假名 | 注音字母 | 谚文相容字母
        // 汉文训读 | 注音符号扩展 | 中日韩笔画 | 片假名拼音扩展 | 带圈中日韩字母和月份
        // 中日韩兼容 | 中日韩统一表意文字扩展区A | 易经六十四卦符号 | 中日韩统一表意文字
        '\u{3000}'..='\u{9FFF}' => true,
        '\u{AC00}'..='\u{D7FF}' => true, // 谚文音节（韩）| 谚文字母扩展-B（韩）
        '\u{F900}'..='\u{FAFF}' => true, // 中日韩相容表意文字 |
        '\u{FE30}'..='\u{FE6F}' => true, // 中日韩相容形式 | 小写变体形式
        '\u{FF00}'..='\u{FFEE}' => true,
        _ => false,
    }
}

fn ends_with_cjk(text: &str) -> bool {
    text.chars().last().map(is_cjk).unwrap_or(false)
}

fn starts_with_cjk(text: &str) -> bool {
    text.chars().next().map(is_cjk).unwrap_or(false)
}

fn find_next_text<'a>(events: &'a [Event], start: usize) -> &'a str {
    for event in events[start..].iter() {
        match event {
            Event::Text(text) => {
                return text;
            }
            _ => continue,
        }
    }

    ""
}

fn find_prev_text<'a>(events: &'a [Event], start: usize) -> &'a str {
    for event in events[..start].iter().rev() {
        match event {
            Event::Text(text) => {
                return text;
            }
            _ => continue,
        }
    }

    ""
}

pub fn join_cjk_spacing(markdown: &str) -> Result<String> {
    // remove SoftBreak if the previous text ends with CJK and next text starts with CJK
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);

    let mut buf = String::with_capacity(markdown.len());

    let mut events: Vec<Event> = Parser::new_ext(markdown, opts).collect();
    let mut keep_event = Vec::with_capacity(events.len());

    for index in 0..events.len() {
        let event = events.get(index).unwrap();
        if *event != Event::SoftBreak {
            keep_event.push(true);
            continue;
        }

        let prev_text = find_prev_text(&events, index);
        let next_text = find_next_text(&events, index);

        if ends_with_cjk(prev_text) && starts_with_cjk(next_text) {
            keep_event.push(false);
        } else {
            keep_event.push(true);
        }
    }

    let mut i = 0;
    events.retain(|_e| {
        let ret = keep_event[i];
        i += 1;
        ret
    });

    cmark(events.into_iter(), &mut buf, None)
        .map(|_| buf)
        .map_err(|err| anyhow!("Markdown serialization failed: {}", err))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn paragraph() {
        assert_eq!(join_cjk_spacing("中文\n测试").unwrap(), "中文测试");
        assert_eq!(join_cjk_spacing("中文\n 测试").unwrap(), "中文测试");
    }

    #[test]
    fn list_items() {
        assert_eq!(join_cjk_spacing("1. 中文\n测试").unwrap(), "1. 中文测试");
        assert_eq!(join_cjk_spacing("1. 中文\n 测试").unwrap(), "1. 中文测试");
    }

    #[test]
    fn list_items_level_2() {
        assert_eq!(
            join_cjk_spacing("1. 中文\n   * 中文\n   测试").unwrap(),
            "1. 中文\n   * 中文测试"
        );
        assert_eq!(
            join_cjk_spacing("1. 中文\n   * 中文\n     测试").unwrap(),
            "1. 中文\n   * 中文测试"
        );
    }

    #[test]
    fn quote() {
        assert_eq!(
            join_cjk_spacing("> 中文\n> 测试").unwrap(),
            "\n > \n > 中文测试"
        );
    }

    #[test]
    fn quote_list() {
        assert_eq!(
            join_cjk_spacing("> 1. 中文\n> 测试").unwrap(),
            "\n > \n > 1. 中文测试"
        );
    }

    #[test]
    fn code_block() {
        // not changed
        assert_eq!(
            join_cjk_spacing("```\n中文\n测试\n```").unwrap(),
            "\n````\n中文\n测试\n````"
        );
        assert_eq!(
            join_cjk_spacing("    中文\n    测试\n").unwrap(),
            "````\n中文\n测试\n````"
        );
    }

    #[test]
    fn footnote() {
        assert_eq!(
            join_cjk_spacing("中文[^foot]\n测试").unwrap(),
            "中文[^foot]测试"
        );
    }

    #[test]
    fn link() {
        assert_eq!(
            join_cjk_spacing("中[Text](http://)\n文").unwrap(),
            "中[Text](http://)\n文"
        );
        assert_eq!(
            join_cjk_spacing("中[链接](http://)\n文").unwrap(),
            "中[链接](http://)文"
        );
    }
}
