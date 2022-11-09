# swc-plugin-translation-converter

swc-plugin-translation-converter is a plugin for [swc](https://swc.rs/) used to convert i18next namespace resource expressions into their serialized representation.

**NOTE:** we don't have the capacity to maintain this all the time, bug support and updates are not guaranteed.

```ts
// i18next setup
import i18n from "i18next";
import { initReactI18next } from "react-i18next";

const l = {
  translationNamespace: {
    helloWorld: "Hello world!",
  },
};

i18n.use(initReactI18next).init({
  resources: {
    en: l,
  },
  lng: "en",
  fallbackLng: "en",
});

// example usage
import React from "react";

const Component = () => {
  const { t } = useTranslation();

  // swc-plugin-translation-converter will convert the below into
  // t("translationNamespace:helloWorld")
  return <div>{t(l.translationNamespace.helloWorld)}</div>;
};
```

## Prerequisites

If you don't already have **swc** setup, you can follow their [getting started guide](https://swc.rs/docs/getting-started).

## Installation

Add **swc-plugin-jsx-remove-attribute** to your dependencies like so:

Yarn v1:

- `yarn add https://github.com/Fullscript/swc-plugin-translation-converter.git#1.0.0`

Yarn v2 (and onwards):

- `yarn add @fullscript/swc-plugin-translation-converter@https://github.com/Fullscript/swc-plugin-translation-converter.git#1.0.0`

NPM:

- `npm install https://github.com/Fullscript/swc-plugin-translation-converter.git#1.0.0`

## Configuration

Wherever your SWC configuration is located, add the following:

```js
{
  jsc: {
    //...
    experimental: {
      plugins: [
        ["@fullscript/swc-plugin-translation-converted", {}],
      ]
    }
  }
}
```

## Contributing

Bug reports and pull requests are welcome :)

### Building for release

1. Run: `cargo build-wasi --release`
2. Copy the resulting wasm file to the root of the project:
`cp ./target/wasm32-wasi/release/swc_plugin_translation_converter.wasm .`
3. Commit and push!
