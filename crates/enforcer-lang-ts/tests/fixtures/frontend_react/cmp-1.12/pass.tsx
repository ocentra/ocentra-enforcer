import Image from "next/image";

export function Avatar({ src }: { src: string }) {
  return <Image src={src} width={48} height={48} alt="User avatar" />;
}
