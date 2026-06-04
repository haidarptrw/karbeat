/// Utility class that simplifies handling errors.
///
/// Return a [Result] from a function to indicate success or failure.
///
/// A [Result] is either an [Ok] with a value of type [T]
/// or an [Error] with an [Exception].
///
/// Use [Result.ok] to create a successful result with a value of type [T].
/// Use [Result.error] to create an error result with an [Exception].
sealed class Result<T> {
  const Result();

  /// Creates an instance of Result containing a value
  factory Result.ok(T value) => Ok(value);

  /// Create an instance of Result containing an error
  factory Result.error(Exception error) => Error(error);

  bool isErr() {
    return this is Error<T>;
  }

  bool isOk() {
    return this is Ok<T>;
  }

  Exception err() {
    return (this as Error<T>).error;
  }

  T ok() {
    return (this as Ok<T>).value;
  }

  @override
  String toString() {
    if (isOk()) {
      return 'Ok(${ok()})';
    } else {
      return 'Error(${err()})';
    }
  }
}

/// Subclass of Result for values
final class Ok<T> extends Result<T> {
  const Ok(this.value);

  /// Returned value in result
  final T value;
}

/// Subclass of Result for errors
final class Error<T> extends Result<T> {
  const Error(this.error);

  /// Returned error in result
  final Exception error;

  String toErrorMessage() {
    return error.toString();
  }
}

/// call a function and handle it in try-catch block.
/// In the end it will return the explicit Result type
/// 
/// Using this lets you define a cleaner
/// scope in which code will throw an error
Result<T> attempt<T>(T Function() fn) {
  try {
    return Result.ok(fn());
  } on Exception catch (e) {
    return Result.error(e);
  } catch (e) {
    return Result.error(Exception(e.toString()));
  }
}

/// call an async function and handle it in try-catch block.
/// In the end it will return the explicit Result type
/// 
/// Using this lets you define a cleaner
/// scope in which code will throw an error (especially
/// in async case where the code have a chance to get an
/// error)
Future<Result<T>> attemptAsync<T>(
  Future<T> Function() fn,
) async {
  try {
    return Result.ok(await fn());
  } on Exception catch (e) {
    return Result.error(e);
  } catch (e) {
    return Result.error(Exception(e.toString()));
  }
}

extension ResultExt<T> on Result<T> {
  Result<U> map<U>(U Function(T value) fn) {
    return switch (this) {
      Ok<T>(value: final value) => Result.ok(fn(value)),
      Error<T>(error: final error) => Result.error(error),
    };
  }

  Result<U> andThen<U>(Result<U> Function(T value) fn) {
    return switch (this) {
      Ok<T>(value: final value) => fn(value),
      Error<T>(error: final error) => Result.error(error),
    };
  }

  Result<T> mapErr(Exception Function(Exception error) fn) {
    return switch (this) {
      Ok<T>() => this,
      Error<T>(error: final error) => Result.error(fn(error)),
    };
  }
}
extension ResultAsyncExt<T> on Future<Result<T>> {
  Future<Result<U>> andThenAsync<U>(
    Future<Result<U>> Function(T value) fn,
  ) async {
    final result = await this;

    return switch (result) {
      Ok<T>(value: final value) => await fn(value),
      Error<T>(error: final error) => Result.error(error),
    };
  }

  Future<Result<U>> mapAsync<U>(
    Future<U> Function(T value) fn,
  ) async {
    final result = await this;

    return switch (result) {
      Ok<T>(value: final value) => Result.ok(await fn(value)),
      Error<T>(error: final error) => Result.error(error),
    };
  }
}