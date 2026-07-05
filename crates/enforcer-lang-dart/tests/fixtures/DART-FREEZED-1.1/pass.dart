// PASS fixture for DART-FREEZED-1.1: immutable @freezed entity.
import 'package:freezed_annotation/freezed_annotation.dart';

@freezed
class Ticket with _$Ticket {
  const factory Ticket({required String status}) = _Ticket;
}
