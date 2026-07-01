export const PortalSmokePeerSource = Object.freeze({ peerId: 'portal-dev', role: 'portal' });

export const PortalSmokeTargets = Object.freeze({
  LocalhostWindowsAgent: Object.freeze({
    deviceId: 'local-dev-agent',
    platform: 'windows',
    route: 'localhost',
  }),
  LocalNetworkWindowsAgent: Object.freeze({
    deviceId: 'local-dev-agent',
    platform: 'windows',
    route: 'local-network',
  }),
});

export function createPortalSmokeCommandEnvelope(
  messageId,
  command,
  payload,
  target = PortalSmokeTargets.LocalhostWindowsAgent
) {
  return {
    schemaVersion: 1,
    messageId,
    sentAt: new Date().toISOString(),
    source: PortalSmokePeerSource,
    target,
    command,
    payload,
  };
}
