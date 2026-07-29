// bad: internal endpoint has no auth guard at all, and trusts the
// x-internal header as its only signal.
router.internal('/internal/settle', (req, res) => {
  if (req.headers['x-internal'] === 'true') {
    settleTransaction(req.body);
  }
  res.send('ok');
});
