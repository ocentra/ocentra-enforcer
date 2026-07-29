"""Vulnerable Flask/Django-style endpoints: mass assignment.

Every handler below binds an entire untrusted request object straight onto
a model/entity with no field allowlist. An attacker who controls the
request body can inject hidden/sensitive fields (role, is_admin, balance,
verified, price, ...) that the handler never intended to accept.
"""

from flask import Flask, jsonify, request

from models import Order, User, db

app = Flask(__name__)


@app.route("/api/users/me", methods=["PUT"])
def update_profile():
    user = User.query.get(get_current_user_id())
    # Binds the ENTIRE request body onto the model with no field allowlist:
    # an attacker can add "role": "admin" or "is_admin": true and escalate.
    user.update(**request.json)
    db.session.commit()
    return jsonify(user.to_dict())


@app.route("/api/register", methods=["POST"])
def register():
    # Same flaw at object-construction time, fed straight into the model.
    new_user = User(**request.get_json())
    db.session.add(new_user)
    db.session.commit()
    return jsonify(new_user.to_dict()), 201


@app.route("/api/orders/<int:order_id>", methods=["PATCH"])
def patch_order(order_id):
    order = Order.query.get(order_id)
    # Hand-rolled equivalent of the double-splat bind: every request key is
    # written onto the object, including hidden fields such as "price".
    for key, value in request.json.items():
        setattr(order, key, value)
    db.session.commit()
    return jsonify(order.to_dict())
