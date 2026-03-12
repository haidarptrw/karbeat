<div align="center">
  <h1>🎵 Karbeat</h1>
  <p><strong>A minimal, cross-platform Digital Audio Workstation (DAW) written in Flutter and Rust.</strong></p>
  <p>
    <a href="https://github.com/haidarptrw/karbeat/stargazers"><img src="https://img.shields.io/github/stars/haidarptrw/karbeat?style=for-the-badge&color=yellow" alt="Stars Badge"/></a>
    <a href="https://github.com/haidarptrw/karbeat/network/members"><img src="https://img.shields.io/github/forks/haidarptrw/karbeat?style=for-the-badge&color=orange" alt="Forks Badge"/></a>
    <a href="https://github.com/haidarptrw/karbeat/issues"><img src="https://img.shields.io/github/issues/haidarptrw/karbeat?style=for-the-badge&color=red" alt="Issues Badge"/></a>
    <a href="https://github.com/haidarptrw/karbeat/blob/main/LICENSE.txt"><img src="https://img.shields.io/github/license/haidarptrw/karbeat?style=for-the-badge&color=blue" alt="License Badge"/></a>
  </p>
</div>

---

Karbeat is a clean, simple, and minimal Digital Audio Workstation (DAW) designed to be cross-platform. We prioritize delivering an effective mobile-first application

By leveraging the performance of [Rust](https://www.rust-lang.org/) for audio processing and the versatile UI capabilities of [Flutter](https://flutter.dev/), Karbeat aims to provide a reliable environment for your musical creativity.

## Features

- **Cross-Platform**: Built for Windows and Linux, with Android support planned in the near future.
- **High-Performance Audio**: Core audio engine built in Rust using [CPAL](https://github.com/RustAudio/cpal).
- **Minimalist Interface**: A clean UI developed with Flutter for a distraction-free workflow.

_More features are currently in active development._

## Tech Stack

- **Frontend**: [Flutter](https://flutter.dev/) & Dart
- **Audio Engine**: [Rust](https://www.rust-lang.org/)
- **Audio Host**: [CPAL (Cross-Platform Audio Library)](https://github.com/RustAudio/cpal)
- **Bridge**: [Flutter Rust Bridge](https://fzyzcjy.github.io/flutter_rust_bridge/)

## 👥 Contributors

A huge thanks to the developers who made this project possible:

| <a href="https://github.com/haidarptrw"><img src="https://github.com/haidarptrw.png" width="80px;" alt=""/></a><br /><sub><b>[haidarptrw](https://github.com/haidarptrw)</b></sub> |
| :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------: |
|                                                                    **Creator, Lead Developer & Audio Engineer**                                                                    |

_(Want to contribute? Check out our developer notes below!)_

## 📚 References & Documentation

Currently, technical documentation and detailed info are still being drafted. Please be patient as we focus on building the core features of the app!

## 👨‍💻 Note for Developers

Thank you for your interest in Karbeat! Here are a few things to keep in mind:

- **Current Focus**: The application is currently prioritizing **Windows** and **Linux** as the primary development environment for faster development. We will focus to complete features to an usable state for users. Eventually we will fully focus on mobile devices support as it is our main priority in the first place
- **Future Platforms**: Once the core application is near completion, we'll implement and optimize features for **Android**.
- **Plugin Host Support**: We will add plugin host and support for VST3, CLAP, and LV2 for Windows, Linux, and MacOS version. As for android, we are still exploring the best fit and possibility of creating our own plugin format.
- **Plugin Development**: Currently our plugin registry is very few. You can help to create a plugin by using the Karbeat Plugin API by implementing the `KarbeatGenerator` or `KarbeatEffect` trait.
---
