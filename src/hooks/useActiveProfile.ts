import { useProfilesContext } from "../contexts/ProfilesContext";



export function useActiveProfile() {

  const {

    activeProfile,

    activeProfileId,

    loading,

    setActiveProfile,

  } = useProfilesContext();



  return {

    activeProfile,

    profileId: activeProfileId,

    loading,

    setActiveProfile,

  };

}

