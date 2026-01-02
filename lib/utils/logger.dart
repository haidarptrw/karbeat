import 'dart:developer' as dev;

enum KarbeatLoggerLogType { info, error, warn }

String _mapLogTypeToString(KarbeatLoggerLogType logType) {
  switch (logType) {
    case KarbeatLoggerLogType.info:
      return "INFO";
    case KarbeatLoggerLogType.error:
      return 'ERROR';
    case KarbeatLoggerLogType.warn:
      return 'WARN';
  }
}

class KarbeatLogger {
  static void log({
    required String message,
    KarbeatLoggerLogType logType = KarbeatLoggerLogType.info,
    int stackFrameOffset = 1,
  }) {
    // Stack frame offset accounts for the call stack:
    // [0] = current line, [1] = log(), [2+] = caller or helper method
    final stackLines = StackTrace.current.toString().split('\n');
    final stackTrace = stackLines[stackFrameOffset];
    final timestamp = DateTime.now().toIso8601String();
    final fileInfo = stackTrace.substring(stackTrace.indexOf('package:'));

    dev.log(
      "[$timestamp $fileInfo] $message",
      name: _mapLogTypeToString(logType),
    );
  }

  static void info(String message) {
    KarbeatLogger.log(
      message: message,
      logType: KarbeatLoggerLogType.info,
      stackFrameOffset: 2,
    );
  }

  static void warn(String message) {
    KarbeatLogger.log(
      message: message,
      logType: KarbeatLoggerLogType.warn,
      stackFrameOffset: 2,
    );
  }

  static void error(String message) {
    KarbeatLogger.log(
      message: message,
      logType: KarbeatLoggerLogType.error,
      stackFrameOffset: 2,
    );
  }
}
