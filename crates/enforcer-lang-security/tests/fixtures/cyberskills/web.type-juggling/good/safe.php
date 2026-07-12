<?php
// Hardened login/verification handlers using strict comparisons,
// hash_equals(), and password_verify().

function authenticate_user(array $users): bool
{
    $username = $_POST['username'];

    if (!isset($users[$username])) {
        return false;
    }

    // Fix 1: password_verify() never performs a loose comparison.
    if (password_verify($_POST['password'], $users[$username]['password'])) {
        return true;
    }

    return false;
}

function verify_reset_token(string $storedToken): bool
{
    $submittedToken = $_POST['token'];

    // Fix 2: hash_equals() is timing-safe and never returns NULL, so an
    // array-shaped $submittedToken cannot be coerced into a match.
    if (is_string($submittedToken) && hash_equals($storedToken, $submittedToken)) {
        return true;
    }

    return false;
}

function verify_backup_code(array $validCodes): bool
{
    // Fix 3: the strict third argument forces type-and-value comparison.
    if (in_array($_GET['code'], $validCodes, true)) {
        return true;
    }

    return false;
}

function verify_api_signature(string $expectedHash): bool
{
    $computedHash = hash('sha256', $_REQUEST['payload']);

    // Fix 4: hash_equals() replaces the loose == comparison.
    if (hash_equals($expectedHash, $computedHash)) {
        return true;
    }

    return false;
}

function verify_role(string $storedRoleHash, string $suppliedRoleHash): bool
{
    // Fix 5: hash_equals() instead of a loose == between two hash values.
    return hash_equals($storedRoleHash, $suppliedRoleHash);
}
