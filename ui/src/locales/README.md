# Adding a language

1. Copy `en.json` to `<language-code>.json` (for example `ja.json` or `pt-BR.json`).
2. Set `meta.name` to the language's own name (for example `日本語`). It is shown in Settings.
3. Translate the values. Keep the keys and the `{{placeholders}}` unchanged.

That is all. The file is picked up automatically at build time and the language appears in
Settings. Missing keys fall back to English. `npm test` reports keys that are missing or
unknown so typos are easy to catch.

---

# 언어 추가하기

1. `en.json`을 `<언어코드>.json`으로 복사합니다 (예: `ja.json`, `pt-BR.json`).
2. `meta.name`에 그 언어의 자기 이름을 적습니다 (예: `日本語`). 설정 화면에 표시됩니다.
3. 값만 번역합니다. 키와 `{{자리표시자}}`는 바꾸지 않습니다.

이것으로 끝입니다. 파일은 빌드할 때 자동으로 인식되어 설정 화면의 언어 목록에 나타납니다.
번역이 빠진 키는 영어로 표시됩니다. `npm test`가 빠진 키와 잘못된 키를 알려 줍니다.
