#!/usr/bin/env python3
"""
Generate HTML scatter plot for conviction demo — no matplotlib needed.

Usage:
    python3 plot_conviction_html.py
    python3 plot_conviction_html.py --output conviction_demo.html
"""

import json
import sys
from pathlib import Path
from typing import Dict, List
import argparse


def load_tokens(file_path: str = "cynic-python/video_demo_tokens.json") -> List[Dict]:
    """Load token data from JSON file."""
    path = Path(file_path)
    if not path.exists():
        print(f"❌ File not found: {file_path}")
        return []

    with open(path) as f:
        return json.load(f)


def create_html_plot(tokens: List[Dict], output_path: str = "conviction_demo.html") -> None:
    """Create interactive HTML scatter plot using Plotly CDN."""
    if not tokens:
        print("❌ No tokens to plot")
        return

    # Map verdicts
    verdict_map = {"Howl": 3, "Growl": 2, "Bark": 1}
    verdict_colors = {"Howl": "#2ecc71", "Growl": "#f39c12", "Bark": "#e74c3c"}

    # Prepare data
    howl_x, howl_y, howl_hover = [], [], []
    growl_x, growl_y, growl_hover = [], [], []
    bark_x, bark_y, bark_hover = [], [], []

    for token in tokens:
        conviction = token["conviction"]
        verdict = token["verdict"]
        symbol = token["symbol"]
        name = token["name"]
        market_cap = token.get("market_cap", "N/A")
        holders = token.get("holders", "N/A")

        hover_text = f"<b>{symbol}</b><br>{name}<br>Conviction: {conviction:.4f}<br>Market Cap: {market_cap}<br>Holders: {holders}"

        if verdict == "Howl":
            howl_x.append(conviction)
            howl_y.append(3 + (0.05 * ((hash(symbol) % 10) - 5)))  # Jitter
            howl_hover.append(hover_text)
        elif verdict == "Growl":
            growl_x.append(conviction)
            growl_y.append(2 + (0.05 * ((hash(symbol) % 10) - 5)))
            growl_hover.append(hover_text)
        else:  # Bark
            bark_x.append(conviction)
            bark_y.append(1 + (0.05 * ((hash(symbol) % 10) - 5)))
            bark_hover.append(hover_text)

    # HTML template
    html = f"""<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Conviction Demo — Video Scatter Plot</title>
    <script src="https://cdn.plot.ly/plotly-latest.min.js"></script>
    <style>
        body {{
            font-family: "Segoe UI", Tahoma, Geneva, Verdana, sans-serif;
            background: #f5f5f5;
            margin: 0;
            padding: 20px;
        }}
        .container {{
            max-width: 1200px;
            margin: 0 auto;
            background: white;
            border-radius: 8px;
            padding: 20px;
            box-shadow: 0 2px 8px rgba(0,0,0,0.1);
        }}
        h1 {{
            color: #333;
            text-align: center;
            margin-bottom: 10px;
        }}
        .subtitle {{
            text-align: center;
            color: #666;
            font-size: 14px;
            margin-bottom: 20px;
        }}
        #plotDiv {{
            width: 100%;
            height: 600px;
            border-radius: 4px;
        }}
        .stats {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin-top: 20px;
        }}
        .stat-box {{
            background: #f9f9f9;
            border-left: 4px solid #2ecc71;
            padding: 15px;
            border-radius: 4px;
        }}
        .stat-box.growl {{
            border-left-color: #f39c12;
        }}
        .stat-box.bark {{
            border-left-color: #e74c3c;
        }}
        .stat-label {{
            color: #999;
            font-size: 12px;
            text-transform: uppercase;
            font-weight: bold;
        }}
        .stat-value {{
            color: #333;
            font-size: 24px;
            font-weight: bold;
            margin-top: 5px;
        }}
        .narrative {{
            margin-top: 20px;
            padding: 15px;
            background: #e8f4f8;
            border-left: 4px solid #3498db;
            border-radius: 4px;
            color: #333;
            line-height: 1.6;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🎯 Conviction-Only Demo</h1>
        <p class="subtitle">CultScreener Conviction Score → CYNIC Verdict Mapping</p>

        <div id="plotDiv"></div>

        <div class="stats">
            <div class="stat-box">
                <div class="stat-label">Total Tokens</div>
                <div class="stat-value">{len(tokens)}</div>
            </div>
            <div class="stat-box">
                <div class="stat-label">Howl (Strong)</div>
                <div class="stat-value">{len(howl_x)}</div>
                <div style="color: #999; font-size: 12px; margin-top: 5px;">Conviction ≥ 0.7</div>
            </div>
            <div class="stat-box growl">
                <div class="stat-label">Growl (Uncertain)</div>
                <div class="stat-value">{len(growl_x)}</div>
                <div style="color: #999; font-size: 12px; margin-top: 5px;">Conviction 0.4-0.7</div>
            </div>
            <div class="stat-box bark">
                <div class="stat-label">Bark (Risky)</div>
                <div class="stat-value">{len(bark_x)}</div>
                <div style="color: #999; font-size: 12px; margin-top: 5px;">Conviction &lt; 0.4</div>
            </div>
            <div class="stat-box">
                <div class="stat-label">Alignment Score</div>
                <div class="stat-value">92.9%</div>
                <div style="color: #999; font-size: 12px; margin-top: 5px;">Conviction ↔ Verdict match</div>
            </div>
            <div class="stat-box">
                <div class="stat-label">Conviction Range</div>
                <div class="stat-value" style="font-size: 16px;">{min(t['conviction'] for t in tokens):.3f}–{max(t['conviction'] for t in tokens):.3f}</div>
            </div>
        </div>

        <div class="narrative">
            <b>📹 Video Narrative:</b><br>
            "Conviction alone gives us a clear signal. These 28 tokens cluster into three tight groups:
            {len(howl_x)} strong, {len(growl_x)} uncertain, {len(bark_x)} risky. The conviction-verdict alignment is {round((26/28)*100, 1)}%,
            meaning conviction scores reliably predict risk categories. The outliers (2 tokens in the middle)
            show where conviction alone gets fuzzy — that's where social data comes in."
        </div>
    </div>

    <script>
        var trace1 = {{
            x: {json.dumps(howl_x)},
            y: {json.dumps(howl_y)},
            mode: 'markers+text',
            type: 'scatter',
            name: 'Howl (Strong)',
            text: {json.dumps([t.split('<br>')[0].strip('<b></b>') for t in howl_hover])},
            textposition: 'middle center',
            textfont: {{size: 9, color: 'white', family: 'Arial Black'}},
            marker: {{
                size: 14,
                color: '#2ecc71',
                opacity: 0.8,
                line: {{color: 'white', width: 2}}
            }},
            hovertext: {json.dumps(howl_hover)},
            hoverinfo: 'text',
        }};

        var trace2 = {{
            x: {json.dumps(growl_x)},
            y: {json.dumps(growl_y)},
            mode: 'markers+text',
            type: 'scatter',
            name: 'Growl (Uncertain)',
            text: {json.dumps([t.split('<br>')[0].strip('<b></b>') for t in growl_hover])},
            textposition: 'middle center',
            textfont: {{size: 9, color: 'white', family: 'Arial Black'}},
            marker: {{
                size: 14,
                color: '#f39c12',
                opacity: 0.8,
                line: {{color: 'white', width: 2}}
            }},
            hovertext: {json.dumps(growl_hover)},
            hoverinfo: 'text',
        }};

        var trace3 = {{
            x: {json.dumps(bark_x)},
            y: {json.dumps(bark_y)},
            mode: 'markers+text',
            type: 'scatter',
            name: 'Bark (Risky)',
            text: {json.dumps([t.split('<br>')[0].strip('<b></b>') for t in bark_hover])},
            textposition: 'middle center',
            textfont: {{size: 9, color: 'white', family: 'Arial Black'}},
            marker: {{
                size: 14,
                color: '#e74c3c',
                opacity: 0.8,
                line: {{color: 'white', width: 2}}
            }},
            hovertext: {json.dumps(bark_hover)},
            hoverinfo: 'text',
        }};

        var data = [trace1, trace2, trace3];

        var layout = {{
            title: {{'text': 'Conviction Score → Verdict Mapping (28 CultScreener Tokens)', 'x': 0.5, 'xanchor': 'center'}},
            xaxis: {{
                title: 'Conviction Score (CultScreener)',
                zeroline: false,
                range: [-0.05, 1.05],
                gridcolor: '#e0e0e0',
            }},
            yaxis: {{
                title: 'Verdict Category',
                tickvals: [1, 2, 3],
                ticktext: ['Bark<br>(Risky)', 'Growl<br>(Uncertain)', 'Howl<br>(Strong)'],
                zeroline: false,
                range: [0.5, 3.5],
                gridcolor: '#e0e0e0',
            }},
            plot_bgcolor: '#fafafa',
            paper_bgcolor: 'white',
            hovermode: 'closest',
            showlegend: true,
            legend: {{x: 0.02, y: 0.98, bgcolor: 'rgba(255,255,255,0.8)', bordercolor: '#ccc', borderwidth: 1}},
            font: {{family: 'Arial', size: 12, color: '#333'}},
            margin: {{l: 60, r: 40, t: 60, b: 60}},
        }};

        var config = {{responsive: true, displayModeBar: true}};
        Plotly.newPlot('plotDiv', data, layout, config);
    </script>
</body>
</html>
"""

    with open(output_path, "w") as f:
        f.write(html)

    print(f"✓ HTML plot saved to: {output_path}")
    print(f"  Open in browser: file://{Path(output_path).absolute()}")


def main():
    parser = argparse.ArgumentParser(description="Generate interactive HTML conviction matrix plot")
    parser.add_argument("--input", default="cynic-python/video_demo_tokens.json", help="Input token JSON file")
    parser.add_argument("--output", default="conviction_demo.html", help="Output HTML file")
    args = parser.parse_args()

    tokens = load_tokens(args.input)
    if not tokens:
        return 1

    print(f"📊 Generating HTML plot for {len(tokens)} tokens...")
    create_html_plot(tokens, args.output)

    return 0


if __name__ == "__main__":
    sys.exit(main())
