import 'dart:developer' as dev;


enum KarbeatLoggerLogType {
  info,
  error,
  warn,
}

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
  static void log({required String message, KarbeatLoggerLogType logType = KarbeatLoggerLogType.info}) {
    final stackTrace = StackTrace.current.toString().split('\n')[1];
    final timestamp = DateTime.now().toIso8601String();
    final fileInfo = stackTrace.substring(stackTrace.indexOf('package:'));

    dev.log(
      "[$timestamp] $message",
      name: _mapLogTypeToString(logType),
      error: 'Source: $fileInfo'
    );
  }

  static void info(String message) {
    KarbeatLogger.log(message: message, logType: KarbeatLoggerLogType.info);
  }

  static void warn(String message) {
    KarbeatLogger.log(message: message, logType: KarbeatLoggerLogType.warn);
  }

  static void error(String message) {
    KarbeatLogger.log(message: message, logType: KarbeatLoggerLogType.error);
  }
}