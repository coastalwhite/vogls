export default function display_bits(size: number, slice: Uint8Array): string {
	let s = `${size}'h`
	let i;
	for (i = 0; i < slice.length; i += 1) {
		s += slice[slice.length - i - 1].toString(16).padStart(2, '0')
	}
	return s;
}
