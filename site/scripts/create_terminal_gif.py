#!/usr/bin/env python3
"""
Create animated GIF with REAL MCP tool calls and responses.
Data sourced from actual Context-Fabric MCP server interactions.
Uses exact response structures from mcp-tool-call-log.json.

v7: Added --font-size and --output parameters
"""
from PIL import Image, ImageDraw, ImageFont
from pathlib import Path
from bidi.algorithm import get_display
import argparse
import os

# Terminal dimensions
WIDTH = 1000
HEIGHT = 750
PADDING = 30

# Colors - brightened for better visibility
BG_COLOR = (18, 18, 22)
TITLE_BAR = (38, 38, 44)
TEXT_COLOR = (245, 242, 235)          # Brighter off-white
ACCENT_COLOR = (220, 185, 128)        # Brighter tan/gold
MUTED_COLOR = (150, 150, 160)         # Significantly brighter gray
USER_COLOR = (160, 195, 255)          # Brighter blue
RESPONSE_COLOR = (160, 220, 160)      # Brighter green
TOOL_COLOR = (220, 185, 128)          # Brighter tan (match accent)
JSON_STRING = (180, 225, 150)         # Brighter green
JSON_KEY = (200, 185, 255)            # Brighter purple
JSON_NUMBER = (255, 200, 150)         # Brighter orange
HEBREW_COLOR = (255, 225, 160)        # Brighter gold

def get_font(size=16):
    font_paths = [
        "/System/Library/Fonts/Monaco.ttf",
        "/System/Library/Fonts/Menlo.ttc",
    ]
    for path in font_paths:
        if os.path.exists(path):
            try:
                return ImageFont.truetype(path, size)
            except:
                pass
    return ImageFont.load_default()

def get_hebrew_font(size=20):
    path = "/System/Library/Fonts/SFHebrew.ttf"
    if os.path.exists(path):
        try:
            return ImageFont.truetype(path, size)
        except:
            pass
    return ImageFont.load_default()

def create_base_frame(title_font_size):
    img = Image.new('RGB', (WIDTH, HEIGHT), BG_COLOR)
    draw = ImageDraw.Draw(img)
    draw.rectangle([0, 0, WIDTH, 36], fill=TITLE_BAR)
    draw.ellipse([14, 12, 26, 24], fill=(255, 95, 86))
    draw.ellipse([36, 12, 48, 24], fill=(255, 189, 46))
    draw.ellipse([58, 12, 70, 24], fill=(39, 201, 63))
    title_font = get_font(title_font_size)
    draw.text((WIDTH//2 - 100, 10), "Context-Fabric MCP Server", fill=MUTED_COLOR, font=title_font)
    return img

def has_hebrew(text):
    for char in text:
        if '\u0590' <= char <= '\u05FF' or '\uFB1D' <= char <= '\uFB4F':
            return True
    return False

def extract_hebrew(text):
    """Extract Hebrew portion from text, return (before, hebrew, after)."""
    import re
    # Match Hebrew characters including vowels and cantillation
    pattern = r'([\u0590-\u05FF\uFB1D-\uFB4F\u0591-\u05C7]+(?:[\s\-־][\u0590-\u05FF\uFB1D-\uFB4F\u0591-\u05C7]+)*)'
    match = re.search(pattern, text)
    if match:
        start, end = match.span()
        return text[:start], match.group(), text[end:]
    return text, '', ''

def render_lines(img, lines, font, hebrew_font, line_height):
    draw = ImageDraw.Draw(img)
    y = 50

    for line in lines:
        if y > HEIGHT - 40:
            break

        text = line.get('text', '')
        color = line.get('color', TEXT_COLOR)
        indent = line.get('indent', 0)
        x = PADDING + indent

        if line.get('type') == 'divider':
            draw.line([(PADDING, y + 10), (WIDTH - PADDING, y + 10)], fill=(60, 60, 70), width=1)
        elif line.get('type') == 'tool':
            draw.text((PADDING, y), "> ", fill=TOOL_COLOR, font=font)
            draw.text((PADDING + 20, y), text, fill=TOOL_COLOR, font=font)
        elif line.get('type') == 'response':
            draw.text((PADDING, y), "< ", fill=RESPONSE_COLOR, font=font)
            draw.text((PADDING + 20, y), "Response:", fill=RESPONSE_COLOR, font=font)
        elif has_hebrew(text):
            # Split into Latin and Hebrew parts, render each with appropriate font
            before, hebrew, after = extract_hebrew(text)
            curr_x = x

            # Render Latin prefix
            if before:
                draw.text((curr_x, y), before, fill=color, font=font)
                bbox = draw.textbbox((0, 0), before, font=font)
                curr_x += bbox[2] - bbox[0]

            # Render Hebrew (with BiDi)
            if hebrew:
                visual_hebrew = get_display(hebrew)
                draw.text((curr_x, y - 2), visual_hebrew, fill=color, font=hebrew_font)
                # Use actual font bbox for width
                bbox = draw.textbbox((0, 0), visual_hebrew, font=hebrew_font)
                curr_x += bbox[2] - bbox[0]

            # Render Latin suffix (strip leading space since we're positioning manually)
            if after:
                draw.text((curr_x, y), after.lstrip(), fill=color, font=font)
        else:
            draw.text((x, y), text, fill=color, font=font)

        y += line_height

    return img

def main():
    parser = argparse.ArgumentParser(description='Generate terminal demo GIF')
    parser.add_argument('--font-size', type=int, default=16, help='Main font size (default: 16)')
    parser.add_argument('--output', type=str, default='demo-terminal.gif', help='Output filename (default: demo-terminal.gif)')
    args = parser.parse_args()

    # Scale line height proportionally with font size
    # Original: font_size=16, line_height=28 (ratio ~1.75)
    line_height = int(args.font_size * 1.75)
    hebrew_font_size = int(args.font_size * 1.125)  # Hebrew slightly larger
    title_font_size = int(args.font_size * 0.8125)  # Title bar smaller

    print(f"Font size: {args.font_size}")
    print(f"Line height: {line_height}")
    print(f"Hebrew font size: {hebrew_font_size}")
    print(f"Output: {args.output}")

    font = get_font(args.font_size)
    hebrew_font = get_hebrew_font(hebrew_font_size)

    all_frames = []
    durations = []  # Per-frame durations in ms

    # Timing constants (in ms)
    TYPING_SPEED = 40        # Fast typing
    LINE_APPEAR = 80         # Each line of response appearing
    BRIEF_PAUSE = 300        # Brief pause after action
    READ_PAUSE = 800         # Time to read a line
    RESPONSE_HOLD = 3500     # Time to read full response
    TITLE_HOLD = 3000        # Title screen
    CLOSING_HOLD = 4000      # Closing CTA

    # ===== SCENE 1: Title =====
    print("Creating Scene 1: Title...")
    img = create_base_frame(title_font_size)
    lines = [
        {'text': ''},
        {'text': '  Context-Fabric MCP Server', 'color': ACCENT_COLOR},
        {'text': '  ----------------------------------------', 'color': MUTED_COLOR},
        {'text': ''},
        {'text': '  Connected: BHSA (Hebrew Bible)', 'color': TEXT_COLOR},
        {'text': '  426,590 words | 88,131 clauses | 253,207 phrases', 'color': MUTED_COLOR},
    ]
    render_lines(img, lines, font, hebrew_font, line_height)
    all_frames.append(img)
    durations.append(TITLE_HOLD)

    # ===== SCENE 2: User question - Verb stem distributions =====
    print("Creating Scene 2: Verb stem query...")
    user_q1 = "User: Compare verb stem distributions across the corpus"

    for i in range(0, len(user_q1) + 1, 3):  # Faster typing (3 chars at a time)
        img = create_base_frame(title_font_size)
        lines = [{'text': ''}, {'text': user_q1[:i] + ("_" if i < len(user_q1) else ""), 'color': USER_COLOR}]
        render_lines(img, lines, font, hebrew_font, line_height)
        all_frames.append(img)
        durations.append(TYPING_SPEED)

    # Ensure final frame shows complete text (in case length not divisible by 3)
    img = create_base_frame(title_font_size)
    lines = [{'text': ''}, {'text': user_q1, 'color': USER_COLOR}]
    render_lines(img, lines, font, hebrew_font, line_height)
    all_frames.append(img)
    durations.append(READ_PAUSE)

    # ===== SCENE 3: Tool call - search with statistics =====
    print("Creating Scene 3: Statistics search...")
    tool_call_1 = [
        {'text': ''},
        {'text': user_q1, 'color': USER_COLOR},
        {'text': ''},
        {'type': 'divider'},
        {'text': 'search', 'type': 'tool'},
        {'text': '  template: "word sp=verb"', 'color': MUTED_COLOR, 'indent': 20},
        {'text': '  return_type: "statistics"', 'color': MUTED_COLOR, 'indent': 20},
        {'text': '  aggregate_features: ["vs", "vt"]', 'color': MUTED_COLOR, 'indent': 20},
    ]

    # Start from user question (i=2) for smooth transition from typing
    for i in range(2, len(tool_call_1) + 1):
        img = create_base_frame(title_font_size)
        render_lines(img, tool_call_1[:i], font, hebrew_font, line_height)
        all_frames.append(img)
        durations.append(LINE_APPEAR if i > 3 else BRIEF_PAUSE)  # Pause on question, then show tool

    # Brief pause before response
    durations[-1] = BRIEF_PAUSE

    # EXACT response from mcp-tool-call-log.json
    response_1 = tool_call_1 + [
        {'type': 'response'},
        {'text': '{', 'color': TEXT_COLOR, 'indent': 20},
        {'text': '  "total_count": 73710,', 'color': JSON_KEY, 'indent': 20},
        {'text': '  "distributions": {', 'color': TEXT_COLOR, 'indent': 20},
        {'text': '    "vs": [', 'color': JSON_STRING, 'indent': 20},
        {'text': '      {"value": "qal", "count": 50205},', 'color': TEXT_COLOR, 'indent': 20},
        {'text': '      {"value": "hif", "count": 9407},', 'color': TEXT_COLOR, 'indent': 20},
        {'text': '      {"value": "piel", "count": 6811}', 'color': TEXT_COLOR, 'indent': 20},
        {'text': '    ],', 'color': TEXT_COLOR, 'indent': 20},
        {'text': '    "vt": [', 'color': JSON_STRING, 'indent': 20},
        {'text': '      {"value": "perf", "count": 21129},', 'color': TEXT_COLOR, 'indent': 20},
        {'text': '      {"value": "impf", "count": 16099},', 'color': TEXT_COLOR, 'indent': 20},
        {'text': '      {"value": "wayq", "count": 14974}', 'color': TEXT_COLOR, 'indent': 20},
        {'text': '    ]', 'color': TEXT_COLOR, 'indent': 20},
        {'text': '  }', 'color': TEXT_COLOR, 'indent': 20},
        {'text': '}', 'color': TEXT_COLOR, 'indent': 20},
    ]

    for i in range(len(tool_call_1), len(response_1) + 1):
        img = create_base_frame(title_font_size)
        render_lines(img, response_1[:i], font, hebrew_font, line_height)
        all_frames.append(img)
        durations.append(LINE_APPEAR)

    # Hold to read response
    durations[-1] = RESPONSE_HOLD

    # ===== SCENE 4: User question - Fronted clauses =====
    print("Creating Scene 4: Fronted clause query...")
    user_q2 = "User: Show fronted clause examples (non-VSO order)"

    base2 = [{'text': '  ...', 'color': MUTED_COLOR, 'indent': 20}, {'text': ''}]

    for i in range(0, len(user_q2) + 1, 3):  # Faster typing
        img = create_base_frame(title_font_size)
        lines = base2 + [{'text': user_q2[:i] + ("_" if i < len(user_q2) else ""), 'color': USER_COLOR}]
        render_lines(img, lines, font, hebrew_font, line_height)
        all_frames.append(img)
        durations.append(TYPING_SPEED)

    # Ensure final frame shows complete text
    img = create_base_frame(title_font_size)
    lines = base2 + [{'text': user_q2, 'color': USER_COLOR}]
    render_lines(img, lines, font, hebrew_font, line_height)
    all_frames.append(img)
    durations.append(READ_PAUSE)

    # ===== SCENE 5: Tool call - fronted clause passages =====
    print("Creating Scene 5: Fronted clause passages...")
    tool_call_2 = [
        {'text': ''},
        {'text': user_q2, 'color': USER_COLOR},
        {'text': ''},
        {'type': 'divider'},
        {'text': 'search', 'type': 'tool'},
        {'text': '  template: "clause typ=xQt0"', 'color': MUTED_COLOR, 'indent': 20},
        {'text': '  return_type: "passages"', 'color': MUTED_COLOR, 'indent': 20},
        {'text': '  limit: 3', 'color': MUTED_COLOR, 'indent': 20},
    ]

    # Start from user question (i=2) for smooth transition from typing
    for i in range(2, len(tool_call_2) + 1):
        img = create_base_frame(title_font_size)
        render_lines(img, tool_call_2[:i], font, hebrew_font, line_height)
        all_frames.append(img)
        durations.append(LINE_APPEAR if i > 3 else BRIEF_PAUSE)

    # Brief pause before response
    durations[-1] = BRIEF_PAUSE

    # Response structure - passages return reference, text, type
    response_2 = tool_call_2 + [
        {'type': 'response'},
        {'text': '{', 'color': TEXT_COLOR, 'indent': 20},
        {'text': '  "total_count": 4879,', 'color': JSON_KEY, 'indent': 20},
        {'text': '  "passages": [', 'color': TEXT_COLOR, 'indent': 20},
        {'text': '    {', 'color': TEXT_COLOR, 'indent': 20},
        {'text': '      "reference": "Genesis 3:12",', 'color': JSON_STRING, 'indent': 20},
        {'text': '      "text": "הִוא נָתְנָה־לִּי מִן־הָעֵץ",', 'color': HEBREW_COLOR, 'indent': 20},
        {'text': '      "type": "clause"', 'color': JSON_STRING, 'indent': 20},
        {'text': '    },', 'color': TEXT_COLOR, 'indent': 20},
        {'text': '    {', 'color': TEXT_COLOR, 'indent': 20},
        {'text': '      "reference": "Genesis 2:17",', 'color': JSON_STRING, 'indent': 20},
        {'text': '      "text": "לֹא תֹאכַל מִמֶּנּוּ",', 'color': HEBREW_COLOR, 'indent': 20},
        {'text': '      "type": "clause"', 'color': JSON_STRING, 'indent': 20},
        {'text': '    },', 'color': TEXT_COLOR, 'indent': 20},
        {'text': '    ...  // 4,877 more', 'color': MUTED_COLOR, 'indent': 20},
        {'text': '  ]', 'color': TEXT_COLOR, 'indent': 20},
        {'text': '}', 'color': TEXT_COLOR, 'indent': 20},
    ]

    for i in range(len(tool_call_2), len(response_2) + 1):
        img = create_base_frame(title_font_size)
        render_lines(img, response_2[:i], font, hebrew_font, line_height)
        all_frames.append(img)
        durations.append(LINE_APPEAR)

    # Hold longer to read Hebrew response
    durations[-1] = RESPONSE_HOLD + 1000  # Extra time for Hebrew

    # ===== SCENE 6: Closing =====
    print("Creating Scene 6: Closing...")
    closing = [
        {'text': ''},
        {'text': '  ----------------------------------------', 'color': ACCENT_COLOR},
        {'text': ''},
        {'text': '  Context-Fabric', 'color': TEXT_COLOR},
        {'text': '  Memory-efficient corpus analysis for AI', 'color': MUTED_COLOR},
        {'text': ''},
        {'text': '  - 65% less memory than Text-Fabric', 'color': MUTED_COLOR},
        {'text': '  - 12x faster corpus loading', 'color': MUTED_COLOR},
        {'text': '  - Native MCP server integration', 'color': MUTED_COLOR},
        {'text': ''},
        {'text': '  pip install context-fabric', 'color': ACCENT_COLOR},
        {'text': ''},
        {'text': '  ----------------------------------------', 'color': ACCENT_COLOR},
    ]

    for i in range(1, len(closing) + 1):
        img = create_base_frame(title_font_size)
        render_lines(img, closing[:i], font, hebrew_font, line_height)
        all_frames.append(img)
        durations.append(LINE_APPEAR * 2)  # Slightly slower reveal

    # Hold on closing CTA
    durations[-1] = CLOSING_HOLD

    # Save GIF
    print(f"\nTotal frames: {len(all_frames)}")
    print(f"Total duration: {sum(durations)/1000:.1f}s")
    output_path = Path(f'/Users/cody/github/context-fabric-site/concepts/assets/{args.output}')

    all_frames[0].save(
        output_path,
        save_all=True,
        append_images=all_frames[1:],
        duration=durations,  # Per-frame durations
        loop=0,
        optimize=True
    )

    print(f"GIF saved to: {output_path}")
    print(f"Size: {WIDTH}x{HEIGHT}")

if __name__ == '__main__':
    main()
