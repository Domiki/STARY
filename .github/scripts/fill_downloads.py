"""Points the download buttons of the site at the files of the newest release.

    python3 .github/scripts/fill_downloads.py site

Run by the workflow that publishes the site, on a copy of the files: the page in the repository keeps
its link to the list of releases, which works without this. A button with data-asset="dmg", "exe" or
"msi" gets the address of the file of that kind; the element with data-version-line gets the sentence
about the current version. Needs the GitHub CLI (`gh`) and GH_TOKEN.
"""
import json
import os
import re
import subprocess
import sys
from pathlib import Path

REPOSITORY = os.environ.get('GITHUB_REPOSITORY', 'Domiki/STARY')
KINDS = {'dmg': '.dmg', 'exe': '-setup.exe', 'msi': '.msi'}


def version_key(tag: str):
    """Orders tags like semantic versions: 0.1.0-pre < 0.1.0 < 0.1.1."""
    match = re.fullmatch(r'v?(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?', tag)
    if not match:
        return None
    major, minor, patch, pre = match.groups()
    return (int(major), int(minor), int(patch), pre is None, pre or '')


def newest_release() -> dict | None:
    text = subprocess.check_output(['gh', 'api', f'repos/{REPOSITORY}/releases?per_page=30'])
    releases = [r for r in json.loads(text) if not r['draft'] and version_key(r['tag_name'])]
    return max(releases, key=lambda r: version_key(r['tag_name']), default=None)


def asset_urls(release: dict) -> dict[str, str]:
    urls = {}
    for kind, suffix in KINDS.items():
        for asset in release['assets']:
            if asset['name'].endswith(suffix):
                urls[kind] = asset['browser_download_url']
    return urls


def patch(html: str, release: dict, urls: dict[str, str]) -> str:
    def retarget(match: re.Match) -> str:
        tag = match.group(0)
        kind = re.search(r'data-asset="(\w+)"', tag).group(1)
        return re.sub(r'href="[^"]*"', f'href="{urls[kind]}"', tag) if kind in urls else tag

    html = re.sub(r'<a\b[^>]*\bdata-asset="\w+"[^>]*>', retarget, html)
    version = release['tag_name'].lstrip('v')
    kind = ', a pre-release' if release['prerelease'] else ''
    sentence = f'The current version is <strong>{version}</strong>{kind}.'
    return re.sub(
        r'(<span data-version-line>).*?(</span>)', lambda m: f'{m.group(1)}{sentence}{m.group(2)}', html
    )


def main() -> int:
    folder = Path(sys.argv[1] if len(sys.argv) > 1 else 'site')
    release = newest_release()
    if release is None:
        print('no release yet: the download page keeps its link to the releases')
        return 0
    urls = asset_urls(release)
    print(release['tag_name'], urls)
    for page in folder.glob('*.html'):
        html = page.read_text(encoding='utf-8')
        patched = patch(html, release, urls)
        if patched != html:
            page.write_text(patched, encoding='utf-8')
            print('patched', page)
    return 0


if __name__ == '__main__':
    sys.exit(main())
