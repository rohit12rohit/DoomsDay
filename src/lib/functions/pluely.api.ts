// FIXED: Removed unused imports (invoke, storage, etc)

// Helper function to check if Pluely API should be used
export async function shouldUsePluelyAPI(): Promise<boolean> {
  // FIXED: Always return false directly for Local Mode
  return false;
}