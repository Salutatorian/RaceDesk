"""Collect the public planner with Scrapling; never execute remote JavaScript."""
import ast
import datetime as dt
import json
import pathlib
import re
import sys
from urllib.parse import urljoin, urlparse

URL = 'https://iracing-week-planner.tmo.lol/'

def embedded_arrays(source):
    arrays = []
    for match in re.finditer(r"JSON\.parse\(('(?:\\.|[^'\\])*')\)", source):
        try:
            value = json.loads(ast.literal_eval(match[1]))
            if isinstance(value, list):
                arrays.append(value)
        except (ValueError, SyntaxError):
            continue
    return arrays

def extract(source):
    arrays = embedded_arrays(source)
    def pick(key):
        return next((a for a in arrays if a and isinstance(a[0], dict) and key in a[0]), None)
    series, cars, tracks = pick('seriesid'), pick('freeWithSubscription'), pick('pkgid')
    if not all((series, cars, tracks)):
        raise ValueError('Planner format changed: schedule or content catalog missing. Existing cache was preserved.')
    for item in series:
        if not all(k in item for k in ('seriesid', 'seriesname', 'start', 'end', 'tracks', 'cars')):
            raise ValueError('Incomplete series in source; existing cache preserved.')
    # The source applies occasional season exceptions. Parse only data, never eval().
    match = re.search(r'\w+=\{(\d+:\{weekStartOffset:.*?)\};const\{duration:', source)
    overrides = {}
    if match:
        literal = '{' + match[1] + '}'
        literal = re.sub(r'([A-Za-z_][A-Za-z_0-9]*|\d+):', r'"\1":', literal)
        literal = re.sub(r'(?<=[\[,])\.(\d)', r'0.\1', literal)
        overrides = json.loads(literal)
    if 'offWeeks:' in source and not overrides:
        raise ValueError('Unable to parse planner week exceptions; refusing an inaccurate refresh.')
    return dict(series=series, cars=cars, tracks=tracks, overrides=overrides,
                fetched_at=dt.datetime.now(dt.timezone.utc).isoformat(), source=URL)

def collect():
    from scrapling.fetchers import Fetcher
    page = Fetcher.get(URL, timeout=30, retries=2)
    if page.status != 200:
        raise RuntimeError(f'Planner returned HTTP {page.status}')
    scripts = page.css('script[src]::attr(src)').getall()
    errors = []
    for script in scripts:
        url = urljoin(URL, script)
        if urlparse(url).netloc != urlparse(URL).netloc:
            continue
        response = Fetcher.get(url, timeout=45, retries=2)
        if response.status != 200:
            continue
        try:
            return extract(response.body.decode('utf-8'))
        except ValueError as error:
            errors.append(str(error))
    raise RuntimeError('; '.join(errors) or 'No schedule bundle found on planner')

if __name__ == '__main__':
    try:
        result = collect()
        target = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path('schedule.json')
        target.parent.mkdir(parents=True, exist_ok=True)
        temp = target.with_suffix('.pending')
        temp.write_text(json.dumps(result, ensure_ascii=False), encoding='utf-8')
        temp.replace(target)
        print(f'Collected {len(result["series"])} series, {len(result["cars"])} cars, {len(result["tracks"])} tracks')
    except Exception as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
