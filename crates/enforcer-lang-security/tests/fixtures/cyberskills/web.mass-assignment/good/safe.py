"""Safe Flask/Django-style endpoints: explicit field allowlists.

Every handler below binds only known-safe, explicitly named fields onto
the model. Hidden/sensitive fields (role, is_admin, balance, ...) can never
reach the model no matter what extra keys an attacker adds to the request
body, because nothing here ever forwards the whole request object.
"""

from flask import Flask, jsonify, request

from models import Order, User, db

ALLOWED_ORDER_FIELDS = {"shipping_address", "notes"}

app = Flask(__name__)


@app.route("/api/users/me", methods=["PUT"])
def update_profile():
    user = User.query.get(get_current_user_id())
    body = request.json
    # Explicit per-field allowlist: only known-safe fields are copied.
    user.update(name=body["name"], email=body["email"])
    db.session.commit()
    return jsonify(user.to_dict())


@app.route("/api/register", methods=["POST"])
def register():
    body = request.get_json()
    # Explicit per-field construction, no double-splat of the whole body.
    new_user = User(name=body["name"], email=body["email"], password=body["password"])
    db.session.add(new_user)
    db.session.commit()
    return jsonify(new_user.to_dict()), 201


@app.route("/api/orders/<int:order_id>", methods=["PATCH"])
def patch_order(order_id):
    order = Order.query.get(order_id)
    # Build a pre-filtered dict from an explicit allowlist first; the loop
    # below iterates over that filtered dict, never over the raw request
    # object's .items().
    safe_updates = {}
    for field_name in ALLOWED_ORDER_FIELDS:
        if field_name in request.json:
            safe_updates[field_name] = request.json[field_name]
    for key, value in safe_updates.items():
        setattr(order, key, value)
    db.session.commit()
    return jsonify(order.to_dict())


@app.route("/api/profile/name", methods=["PATCH"])
def patch_name():
    user = User.query.get(get_current_user_id())
    # Single-field access, not a whole-object bind.
    user.name = request.json["name"]
    db.session.commit()
    return jsonify(user.to_dict())
