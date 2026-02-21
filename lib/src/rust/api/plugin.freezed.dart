// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'plugin.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$UiEffectTarget {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiEffectTarget);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'UiEffectTarget()';
}


}

/// @nodoc
class $UiEffectTargetCopyWith<$Res>  {
$UiEffectTargetCopyWith(UiEffectTarget _, $Res Function(UiEffectTarget) __);
}


/// Adds pattern-matching-related methods to [UiEffectTarget].
extension UiEffectTargetPatterns on UiEffectTarget {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( UiEffectTarget_Track value)?  track,TResult Function( UiEffectTarget_Master value)?  master,TResult Function( UiEffectTarget_Bus value)?  bus,required TResult orElse(),}){
final _that = this;
switch (_that) {
case UiEffectTarget_Track() when track != null:
return track(_that);case UiEffectTarget_Master() when master != null:
return master(_that);case UiEffectTarget_Bus() when bus != null:
return bus(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( UiEffectTarget_Track value)  track,required TResult Function( UiEffectTarget_Master value)  master,required TResult Function( UiEffectTarget_Bus value)  bus,}){
final _that = this;
switch (_that) {
case UiEffectTarget_Track():
return track(_that);case UiEffectTarget_Master():
return master(_that);case UiEffectTarget_Bus():
return bus(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( UiEffectTarget_Track value)?  track,TResult? Function( UiEffectTarget_Master value)?  master,TResult? Function( UiEffectTarget_Bus value)?  bus,}){
final _that = this;
switch (_that) {
case UiEffectTarget_Track() when track != null:
return track(_that);case UiEffectTarget_Master() when master != null:
return master(_that);case UiEffectTarget_Bus() when bus != null:
return bus(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( int field0)?  track,TResult Function()?  master,TResult Function( int field0)?  bus,required TResult orElse(),}) {final _that = this;
switch (_that) {
case UiEffectTarget_Track() when track != null:
return track(_that.field0);case UiEffectTarget_Master() when master != null:
return master();case UiEffectTarget_Bus() when bus != null:
return bus(_that.field0);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( int field0)  track,required TResult Function()  master,required TResult Function( int field0)  bus,}) {final _that = this;
switch (_that) {
case UiEffectTarget_Track():
return track(_that.field0);case UiEffectTarget_Master():
return master();case UiEffectTarget_Bus():
return bus(_that.field0);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( int field0)?  track,TResult? Function()?  master,TResult? Function( int field0)?  bus,}) {final _that = this;
switch (_that) {
case UiEffectTarget_Track() when track != null:
return track(_that.field0);case UiEffectTarget_Master() when master != null:
return master();case UiEffectTarget_Bus() when bus != null:
return bus(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class UiEffectTarget_Track extends UiEffectTarget {
  const UiEffectTarget_Track(this.field0): super._();
  

 final  int field0;

/// Create a copy of UiEffectTarget
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiEffectTarget_TrackCopyWith<UiEffectTarget_Track> get copyWith => _$UiEffectTarget_TrackCopyWithImpl<UiEffectTarget_Track>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiEffectTarget_Track&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'UiEffectTarget.track(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $UiEffectTarget_TrackCopyWith<$Res> implements $UiEffectTargetCopyWith<$Res> {
  factory $UiEffectTarget_TrackCopyWith(UiEffectTarget_Track value, $Res Function(UiEffectTarget_Track) _then) = _$UiEffectTarget_TrackCopyWithImpl;
@useResult
$Res call({
 int field0
});




}
/// @nodoc
class _$UiEffectTarget_TrackCopyWithImpl<$Res>
    implements $UiEffectTarget_TrackCopyWith<$Res> {
  _$UiEffectTarget_TrackCopyWithImpl(this._self, this._then);

  final UiEffectTarget_Track _self;
  final $Res Function(UiEffectTarget_Track) _then;

/// Create a copy of UiEffectTarget
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(UiEffectTarget_Track(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class UiEffectTarget_Master extends UiEffectTarget {
  const UiEffectTarget_Master(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiEffectTarget_Master);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'UiEffectTarget.master()';
}


}




/// @nodoc


class UiEffectTarget_Bus extends UiEffectTarget {
  const UiEffectTarget_Bus(this.field0): super._();
  

 final  int field0;

/// Create a copy of UiEffectTarget
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiEffectTarget_BusCopyWith<UiEffectTarget_Bus> get copyWith => _$UiEffectTarget_BusCopyWithImpl<UiEffectTarget_Bus>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiEffectTarget_Bus&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'UiEffectTarget.bus(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $UiEffectTarget_BusCopyWith<$Res> implements $UiEffectTargetCopyWith<$Res> {
  factory $UiEffectTarget_BusCopyWith(UiEffectTarget_Bus value, $Res Function(UiEffectTarget_Bus) _then) = _$UiEffectTarget_BusCopyWithImpl;
@useResult
$Res call({
 int field0
});




}
/// @nodoc
class _$UiEffectTarget_BusCopyWithImpl<$Res>
    implements $UiEffectTarget_BusCopyWith<$Res> {
  _$UiEffectTarget_BusCopyWithImpl(this._self, this._then);

  final UiEffectTarget_Bus _self;
  final $Res Function(UiEffectTarget_Bus) _then;

/// Create a copy of UiEffectTarget
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(UiEffectTarget_Bus(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

// dart format on
