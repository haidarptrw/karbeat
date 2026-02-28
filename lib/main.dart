import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:karbeat/app.dart';
import 'package:karbeat/src/rust/frb_generated.dart';

Future<void> main() async {
  await RustLib.init();

  runApp(const ProviderScope(child: KarbeatApp()));
}
