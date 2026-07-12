<?php
// Legacy login/verification handlers pending migration to strict
// comparisons, hash_equals(), and password_verify().

function authenticate_user(array $users): bool
{
    $username = $_POST['username'];

    if (!isset($users[$username])) {
        return false;
    }

    // Bug 1: loose comparison directly against $_POST lets password=0 or
    // password=true bypass a non-empty stored password (PHP type juggling).
    if ($_POST['password'] == $users[$username]['password']) {
        return true;
    }

    return false;
}

function verify_reset_token(string $storedToken): bool
{
    // Bug 2: strcmp() returns NULL when given an array (token[]=x), and
    // NULL == 0 in PHP's loose comparison, so sending the token as an array
    // bypasses this check entirely.
    if (strcmp($_POST['token'], $storedToken) == 0) {
        return true;
    }

    return false;
}

function verify_backup_code(array $validCodes): bool
{
    // Bug 3: in_array() without the strict third argument lets a
    // magic-hash-shaped string loosely match a numeric valid code.
    if (in_array($_GET['code'], $validCodes)) {
        return true;
    }

    return false;
}

function verify_api_signature(string $expectedHash): bool
{
    // Bug 4: md5() compared with a loose operator is vulnerable to "0e..."
    // magic-hash collisions (two distinct inputs whose digests both look
    // like scientific notation are then treated as identical).
    if (md5($_REQUEST['payload']) == $expectedHash) {
        return true;
    }

    return false;
}

function verify_role(string $storedRoleHash, string $suppliedRoleHash): bool
{
    // Bug 5: comparing two hash-looking values with a loose == still
    // permits a magic-hash collision even when neither call is inline here.
    if ($storedRoleHash == $suppliedRoleHash) {
        return true;
    }

    return false;
}
