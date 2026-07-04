// good: internal endpoint verifies the service token before honoring
// anything; the internal header is never treated as the sole signal.
router.internal('/internal/settle', (req, res) => {
  verifyServiceToken(req);
  settleTransaction(req.body);
  res.send('ok');
});
